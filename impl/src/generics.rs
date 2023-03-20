use std::collections::{btree_map::Entry as MapEntry, BTreeMap as Map};
use syn::{
    Expr, GenericArgument, GenericParam, Generics, Ident, Lifetime, PathArguments, TraitBound,
    Type, WherePredicate,
};

#[derive(Default)]
pub struct GenericsAnalyzer<'a> {
    pub bounds: GenericsMap<'a>,
    pub extra_bounds: Vec<&'a WherePredicate>,
}

#[derive(Default)]
pub struct GenericsMap<'a> {
    indices: Map<GenericName<'a>, usize>,
    entries: Vec<(GenericName<'a>, GenericBounds<'a>)>,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum GenericName<'a> {
    Ident(&'a Ident),
    Lifetime(&'a Lifetime),
}

impl<'a> From<&'a Ident> for GenericName<'a> {
    fn from(t: &'a Ident) -> Self {
        GenericName::Ident(t)
    }
}

impl<'a> From<&'a Lifetime> for GenericName<'a> {
    fn from(t: &'a Lifetime) -> Self {
        GenericName::Lifetime(t)
    }
}

#[derive(Default)]
pub struct GenericBounds<'a> {
    pub params: Vec<TypeParamBound<'a>>,
    pub const_ty: Option<&'a Type>,
    pub selected: bool,
}

#[derive(Clone, Copy)]
pub enum TypeParamBound<'a> {
    Trait(&'a TraitBound),
    Lifetime(&'a Lifetime),
}

impl<'a> From<&'a TraitBound> for TypeParamBound<'a> {
    fn from(t: &'a TraitBound) -> Self {
        TypeParamBound::Trait(t)
    }
}

impl<'a> From<&'a Lifetime> for TypeParamBound<'a> {
    fn from(t: &'a Lifetime) -> Self {
        TypeParamBound::Lifetime(t)
    }
}

impl<'a> From<&'a syn::TypeParamBound> for TypeParamBound<'a> {
    fn from(t: &'a syn::TypeParamBound) -> Self {
        match t {
            syn::TypeParamBound::Trait(t) => TypeParamBound::Trait(t),
            syn::TypeParamBound::Lifetime(t) => TypeParamBound::Lifetime(t),
        }
    }
}

impl<'a> GenericsAnalyzer<'a> {
    pub fn intersects(
        &mut self,
        ty: &'a Type,
        cb: impl FnMut(GenericName<'a>, &mut GenericBounds<'a>),
    ) {
        ImplIntersects {
            analyzer: self,
            cb: Box::new(cb),
        }
        .ty(ty);
    }

    pub fn from_syn(generics: &'a Generics) -> Self {
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
    {
        match self.bounds.indices.entry(name.into()) {
            MapEntry::Vacant(_) => self.extra_bounds.push(predicate),
            MapEntry::Occupied(v) => self
                .bounds
                .entries
                .get_mut(*v.get())
                .unwrap()
                .1
                .params
                .extend(bounds.map(T::into)),
        }
    }

    fn update_params<N, T>(&mut self, name: N, bounds: impl Iterator<Item = T>)
    where
        N: Into<GenericName<'a>>,
        T: Into<TypeParamBound<'a>>,
    {
        self.bounds
            .insert_or_default(name.into())
            .params
            .extend(bounds.map(T::into));
    }
}

struct ImplIntersects<'a, 'b> {
    analyzer: &'b mut GenericsAnalyzer<'a>,
    cb: Box<dyn 'b + FnMut(GenericName<'a>, &mut GenericBounds<'a>)>,
}

impl<'a, 'b> ImplIntersects<'a, 'b> {
    fn ty(&mut self, ty: &'a Type) {
        match ty {
            Type::Array(ty) => {
                self.ty(&ty.elem);
                self.expr(&ty.len);
            }
            Type::BareFn(ty) => {
                for arg in ty.inputs.iter() {
                    self.ty(&arg.ty);
                }
                self.return_ty(&ty.output);
            }
            Type::Group(ty) => self.ty(&ty.elem),
            Type::Paren(ty) => self.ty(&ty.elem),
            Type::Path(ty) => self.path(ty.qself.is_none(), &ty.path),
            Type::Ptr(ty) => self.ty(&ty.elem),
            Type::Reference(ty) => {
                if let Some(lt) = ty.lifetime.as_ref() {
                    self.callback(lt);
                }
                self.ty(&ty.elem);
            }
            Type::Slice(ty) => self.ty(&ty.elem),
            Type::TraitObject(ty) => {
                for bound in ty.bounds.iter() {
                    match bound {
                        syn::TypeParamBound::Trait(ty) => self.path(true, &ty.path),
                        syn::TypeParamBound::Lifetime(lt) => self.callback(lt),
                    }
                }
            }
            Type::Tuple(ty) => {
                for ty in ty.elems.iter() {
                    self.ty(ty);
                }
            }
            _ => (),
        }
    }

    fn expr(&mut self, expr: &'a Expr) {
        if let Expr::Path(ty) = expr {
            self.path(ty.qself.is_none(), &ty.path);
        }
    }

    fn path(&mut self, check_ident: bool, path: &'a syn::Path) {
        if check_ident {
            if let Some(ident) = path.get_ident() {
                self.callback(ident);
            }
        }
        for segment in path.segments.iter() {
            match &segment.arguments {
                PathArguments::AngleBracketed(arguments) => {
                    for arg in arguments.args.iter() {
                        match arg {
                            GenericArgument::Lifetime(lt) => self.callback(lt),
                            GenericArgument::Type(ty) => self.ty(ty),
                            GenericArgument::Const(expr) => self.expr(expr),
                            GenericArgument::Binding(ty) => self.ty(&ty.ty),
                            _ => (),
                        }
                    }
                }
                PathArguments::Parenthesized(arguments) => {
                    for ty in arguments.inputs.iter() {
                        self.ty(ty);
                    }
                    self.return_ty(&arguments.output);
                }
                _ => (),
            }
        }
    }

    fn return_ty(&mut self, ty: &'a syn::ReturnType) {
        if let syn::ReturnType::Type(_, ty) = &ty {
            self.ty(ty);
        }
    }

    fn callback(&mut self, name: impl Into<GenericName<'a>>) {
        if let Some((key, bounds)) = self.analyzer.bounds.get_mut(name) {
            (self.cb)(key, bounds);
        }
    }
}

impl<'a> GenericsMap<'a> {
    pub fn iter(&self) -> impl Iterator<Item = (GenericName<'a>, &GenericBounds<'a>)> {
        self.entries.iter().map(|(name, bounds)| (*name, bounds))
    }

    fn insert_or_default<N>(&mut self, name: N) -> &mut GenericBounds<'a>
    where
        N: Into<GenericName<'a>>,
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
        &mut self.entries.get_mut(index).unwrap().1
    }

    pub fn get_mut<N>(&mut self, name: N) -> Option<(GenericName<'a>, &mut GenericBounds<'a>)>
    where
        N: Into<GenericName<'a>>,
    {
        if let Some(index) = self.indices.get(&name.into()) {
            self.entries
                .get_mut(*index)
                .map(|(name, bounds)| (*name, bounds))
        } else {
            None
        }
    }
}
