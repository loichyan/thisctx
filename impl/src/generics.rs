use std::collections::{btree_map::Entry as MapEntry, BTreeMap as Map};
use syn::{
    GenericArgument, GenericParam, Generics, Ident, Lifetime, PathArguments, TraitBound, Type,
    WherePredicate,
};

pub struct GenericsAnalyzer<'a, C> {
    pub bounds: GenericsMap<'a, C>,
    pub extra_bounds: Vec<&'a WherePredicate>,
}

impl<C> Default for GenericsAnalyzer<'_, C> {
    fn default() -> Self {
        Self {
            bounds: <_>::default(),
            extra_bounds: <_>::default(),
        }
    }
}

pub struct GenericsMap<'a, C> {
    indices: Map<GenericName<'a>, usize>,
    entries: Vec<(GenericName<'a>, GenericBounds<'a, C>)>,
}

impl<C> Default for GenericsMap<'_, C> {
    fn default() -> Self {
        Self {
            indices: <_>::default(),
            entries: <_>::default(),
        }
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum GenericName<'a> {
    Ident(&'a Ident),
    Lifetime(&'a Lifetime),
}

impl<'a> From<&'a Ident> for GenericName<'a> {
    fn from(t: &'a Ident) -> Self {
        Self::Ident(t)
    }
}

impl<'a> From<&'a Lifetime> for GenericName<'a> {
    fn from(t: &'a Lifetime) -> Self {
        Self::Lifetime(t)
    }
}

#[derive(Default)]
pub struct GenericBounds<'a, C> {
    pub params: Vec<TypeParamBound<'a>>,
    pub const_ty: Option<&'a Type>,
    pub context: C,
}

#[derive(Clone, Copy)]
pub enum TypeParamBound<'a> {
    Trait(&'a TraitBound),
    Lifetime(&'a Lifetime),
}

impl<'a> From<&'a TraitBound> for TypeParamBound<'a> {
    fn from(t: &'a TraitBound) -> Self {
        Self::Trait(t)
    }
}

impl<'a> From<&'a Lifetime> for TypeParamBound<'a> {
    fn from(t: &'a Lifetime) -> Self {
        Self::Lifetime(t)
    }
}

impl<'a> From<&'a syn::TypeParamBound> for TypeParamBound<'a> {
    fn from(t: &'a syn::TypeParamBound) -> Self {
        match t {
            syn::TypeParamBound::Trait(t) => Self::Trait(t),
            syn::TypeParamBound::Lifetime(t) => Self::Lifetime(t),
        }
    }
}

impl<'a, C> GenericsAnalyzer<'a, C> {
    pub fn intersects(
        &mut self,
        ty: &'a Type,
        mut cb: impl FnMut(GenericName<'a>, &mut GenericBounds<'a, C>),
    ) {
        self.intersects_impl(ty, &mut cb);
    }

    fn intersects_impl(
        &mut self,
        ty: &'a Type,
        cb: &mut impl FnMut(GenericName<'a>, &mut GenericBounds<'a, C>),
    ) {
        match ty {
            Type::Path(ty) => {
                if ty.qself.is_none() {
                    if let Some(ident) = ty.path.get_ident() {
                        self.intersects_callback(ident, &mut *cb);
                    }
                }
                for segment in ty.path.segments.iter() {
                    if let PathArguments::AngleBracketed(arguments) = &segment.arguments {
                        for arg in arguments.args.iter() {
                            match arg {
                                GenericArgument::Type(ty) => self.intersects_impl(ty, cb),
                                GenericArgument::Lifetime(lt) => {
                                    self.intersects_callback(lt, &mut *cb);
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
            Type::Reference(ty) => {
                if let Some(lt) = ty.lifetime.as_ref() {
                    self.intersects_callback(lt, &mut *cb);
                }
                self.intersects_impl(&ty.elem, cb);
            }
            _ => (),
        }
    }

    fn intersects_callback(
        &mut self,
        name: impl Into<GenericName<'a>>,
        cb: impl FnOnce(GenericName<'a>, &mut GenericBounds<'a, C>),
    ) {
        if let Some((key, bounds)) = self.bounds.get_mut(name) {
            cb(key, bounds);
        }
    }

    pub fn from_syn(generics: &'a Generics) -> Self
    where
        C: Default,
    {
        let mut new = Self::default();
        // Collect bounds from type parameter.
        for param in generics.params.iter() {
            match param {
                GenericParam::Type(ty) => new.update_params(&ty.ident, ty.bounds.iter()),
                GenericParam::Lifetime(lt) => new.update_params(&lt.lifetime, lt.bounds.iter()),
                GenericParam::Const(kst) => {
                    new.bounds.insert_or_default(&kst.ident).const_ty = Some(&kst.ty)
                }
            }
        }
        // Collect bounds from where clause.
        if let Some(clause) = generics.where_clause.as_ref() {
            for predicate in clause.predicates.iter() {
                match predicate {
                    WherePredicate::Type(ty) => match &ty.bounded_ty {
                        Type::Path(path) if path.qself.is_none() => {
                            if let Some(ident) = path.path.get_ident() {
                                new.update_clause(ident, ty.bounds.iter(), predicate);
                            } else {
                                new.extra_bounds.push(predicate);
                            }
                        }
                        _ => new.extra_bounds.push(predicate),
                    },
                    WherePredicate::Lifetime(lt) => {
                        new.update_clause(&lt.lifetime, lt.bounds.iter(), predicate);
                    }
                    WherePredicate::Eq(_) => new.extra_bounds.push(predicate),
                }
            }
        }
        new
    }

    fn update_clause<N, T>(
        &mut self,
        name: N,
        bounds: impl Iterator<Item = T>,
        predicate: &'a WherePredicate,
    ) where
        N: Into<GenericName<'a>>,
        T: Into<TypeParamBound<'a>>,
        C: Default,
    {
        match self.bounds.indices.entry(name.into()) {
            MapEntry::Vacant(_) => self.extra_bounds.push(predicate),
            MapEntry::Occupied(v) => self
                .bounds
                .entries
                .get_mut(*v.get())
                .unwrap_or_else(|| unreachable!())
                .1
                .params
                .extend(bounds.map(T::into)),
        }
    }

    fn update_params<N, T>(&mut self, name: N, bounds: impl Iterator<Item = T>)
    where
        N: Into<GenericName<'a>>,
        T: Into<TypeParamBound<'a>>,
        C: Default,
    {
        self.bounds
            .insert_or_default(name.into())
            .params
            .extend(bounds.map(T::into));
    }
}

impl<'a, C> GenericsMap<'a, C> {
    pub fn iter(&self) -> impl Iterator<Item = (GenericName<'a>, &GenericBounds<'a, C>)> {
        self.entries.iter().map(|(name, bounds)| (*name, bounds))
    }

    fn insert_or_default<N>(&mut self, name: N) -> &mut GenericBounds<'a, C>
    where
        N: Into<GenericName<'a>>,
        C: Default,
    {
        let name = name.into();
        let index;
        match self.indices.entry(name) {
            MapEntry::Occupied(v) => index = *v.get(),
            MapEntry::Vacant(_) => {
                index = self.entries.len();
                self.indices.insert(name, index);
                self.entries.push((name, <_>::default()));
            }
        };
        &mut self
            .entries
            .get_mut(index)
            .unwrap_or_else(|| unreachable!())
            .1
    }

    pub fn get_mut<N>(&mut self, name: N) -> Option<(GenericName<'a>, &mut GenericBounds<'a, C>)>
    where
        N: Into<GenericName<'a>>,
    {
        self.indices
            .get(&name.into())
            .map(|index| {
                self.entries
                    .get_mut(*index)
                    .unwrap_or_else(|| unreachable!())
            })
            .map(|(name, bounds)| (*name, bounds))
    }
}
