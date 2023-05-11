use std::collections::{btree_map::Entry, BTreeMap as Map};
use syn::{
    Expr, GenericArgument, GenericParam, Generics, Ident, Lifetime, Path, PathArguments, QSelf,
    ReturnType, TraitBound, Type, TypeParamBound, WherePredicate,
};

#[derive(Default)]
pub struct ContainerGenerics<'a> {
    generics: Map<GenericName<'a>, usize>,
    orders: Vec<GenericInfo<'a>>,
    /// Bounds that not belong to any container generic.
    pub extra_bounds: Vec<&'a WherePredicate>,
}

pub struct GenericInfo<'a> {
    pub order: usize,
    pub name: GenericName<'a>,
    /// Bounds of this generic, including those from both the definition and the
    /// where clause.
    pub bounds: Vec<GenericBound<'a>>,
    /// Type of a const generic.
    pub const_ty: Option<&'a Type>,
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum GenericName<'a> {
    Ident(&'a Ident),
    Lifetime(&'a Lifetime),
}

impl<'a> From<&'a Ident> for GenericName<'a> {
    fn from(value: &'a Ident) -> Self {
        Self::Ident(value)
    }
}

impl<'a> From<&'a Lifetime> for GenericName<'a> {
    fn from(value: &'a Lifetime) -> Self {
        Self::Lifetime(value)
    }
}

#[derive(Clone, Copy)]
pub enum GenericBound<'a> {
    Trait(&'a TraitBound),
    Lifetime(&'a Lifetime),
}

impl<'a> GenericBound<'a> {
    fn from_bound(bound: &'a TypeParamBound) -> Option<Self> {
        match bound {
            TypeParamBound::Trait(ty) => Some(Self::Trait(ty)),
            TypeParamBound::Lifetime(lt) => Some(Self::Lifetime(lt)),
            _ => None,
        }
    }
}

impl<'a> From<&'a TraitBound> for GenericBound<'a> {
    fn from(value: &'a TraitBound) -> Self {
        Self::Trait(value)
    }
}

impl<'a> From<&'a Lifetime> for GenericBound<'a> {
    fn from(value: &'a Lifetime) -> Self {
        Self::Lifetime(value)
    }
}

impl<'a> ContainerGenerics<'a> {
    pub fn get<'b, 'c>(&'b self, name: impl Into<GenericName<'c>>) -> Option<&'b GenericInfo<'a>> {
        self.generics.get(&name.into()).map(|&i| &self.orders[i])
    }

    pub fn iter(&self) -> impl Iterator<Item = &GenericInfo<'a>> {
        self.orders.iter()
    }

    pub fn from_syn(generics: &'a Generics) -> Self {
        let mut new = Self::default();
        new.update_from_syn(generics);
        new
    }

    fn update_from_syn(&mut self, generics: &'a Generics) {
        // Update bounds from generic definitions.
        for param in generics.params.iter() {
            match param {
                GenericParam::Type(ty) => self.update_bounds(
                    &ty.ident,
                    ty.bounds.iter().filter_map(GenericBound::from_bound),
                ),
                GenericParam::Lifetime(lt) => {
                    self.update_bounds(&lt.lifetime, lt.bounds.iter().map(GenericBound::from))
                }
                GenericParam::Const(kst) => {
                    self.get_or_default(&kst.ident).const_ty = Some(&kst.ty)
                }
            }
        }

        if let Some(clause) = &generics.where_clause {
            // Find bounds belong to defined generics.
            for predicate in clause.predicates.iter() {
                match predicate {
                    WherePredicate::Type(pred) => {
                        // Find all generics that appear in this bound.
                        let intersection = self
                            .intersection(&pred.bounded_ty)
                            .into_iter()
                            .map(|ty| ty.name)
                            .collect::<Vec<_>>();
                        if intersection.is_empty() {
                            self.extra_bounds.push(predicate);
                        } else {
                            for name in intersection.into_iter() {
                                self.update_bounds(
                                    name,
                                    pred.bounds.iter().filter_map(GenericBound::from_bound),
                                );
                            }
                        }
                    }
                    WherePredicate::Lifetime(pred) => {
                        if let Some(info) = self.get_mut(&pred.lifetime) {
                            info.bounds
                                .extend(pred.bounds.iter().map(GenericBound::from));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn update_bounds(
        &mut self,
        name: impl Into<GenericName<'a>>,
        bounds: impl IntoIterator<Item = GenericBound<'a>>,
    ) {
        self.get_or_default(name.into()).bounds.extend(bounds)
    }

    fn get_or_default(&mut self, name: impl Into<GenericName<'a>>) -> &mut GenericInfo<'a> {
        let name = name.into();
        let index;
        match self.generics.entry(name) {
            Entry::Occupied(val) => index = *val.get(),
            Entry::Vacant(entry) => {
                index = self.orders.len();
                entry.insert(index);
                self.orders.push(GenericInfo {
                    order: index,
                    name,
                    bounds: Default::default(),
                    const_ty: None,
                });
            }
        }
        &mut self.orders[index]
    }

    fn get_mut(&mut self, name: impl Into<GenericName<'a>>) -> Option<&mut GenericInfo<'a>> {
        self.generics
            .get(&name.into())
            .map(|&i| &mut self.orders[i])
    }

    /// Collects generics used in the given type.
    pub fn intersection<'b>(&'b self, ty: &Type) -> Vec<&'b GenericInfo<'a>> {
        let mut crawler = Crawler {
            generics: self,
            collection: Default::default(),
        };
        crawler.crawl(ty);
        crawler.collection
    }
}

struct Crawler<'a, 'b> {
    generics: &'b ContainerGenerics<'a>,
    collection: Vec<&'b GenericInfo<'a>>,
}

impl<'a, 'b> Crawler<'a, 'b> {
    fn crawl(&mut self, ty: &Type) {
        match ty {
            Type::Array(a) => {
                self.crawl(&a.elem);
                self.crawl_expr(&a.len);
            }
            Type::BareFn(f) => {
                self.crawl_fn(f.inputs.iter().map(|arg| &arg.ty), &f.output);
            }
            Type::Group(g) => self.crawl(&g.elem),
            Type::ImplTrait(i) => i.bounds.iter().for_each(|b| self.crawl_bound(b)),
            Type::Paren(p) => self.crawl(&p.elem),
            Type::Path(p) => self.crawl_path(p.qself.as_ref(), &p.path),
            Type::Ptr(p) => self.crawl(&p.elem),
            Type::Reference(r) => {
                if let Some(lt) = &r.lifetime {
                    self.try_collect(lt);
                }
                self.crawl(&r.elem);
            }
            Type::Slice(s) => self.crawl(&s.elem),
            Type::TraitObject(t) => t.bounds.iter().for_each(|b| self.crawl_bound(b)),
            Type::Tuple(t) => t.elems.iter().for_each(|ty| self.crawl(&ty)),
            _ => {}
        }
    }

    fn crawl_path(&mut self, qself: Option<&QSelf>, path: &Path) {
        if let Some(qself) = &qself {
            self.crawl(&qself.ty);
        } else if let Some(ident) = path.get_ident() {
            self.try_collect(ident);
        }
        path.segments.iter().for_each(|seg| match &seg.arguments {
            PathArguments::AngleBracketed(a) => a.args.iter().for_each(|arg| match arg {
                GenericArgument::Lifetime(lt) => self.try_collect(lt),
                GenericArgument::Type(ty) => self.crawl(ty),
                GenericArgument::Const(kst) => self.crawl_expr(kst),
                GenericArgument::AssocType(a) => self.crawl(&a.ty),
                GenericArgument::AssocConst(a) => self.crawl_expr(&a.value),
                _ => {}
            }),
            PathArguments::Parenthesized(p) => {
                self.crawl_fn(p.inputs.iter(), &p.output);
            }
            PathArguments::None => {}
        });
    }

    fn crawl_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Path(p) => self.crawl_path(p.qself.as_ref(), &p.path),
            _ => {}
        }
    }

    fn crawl_fn<'c>(&mut self, inputs: impl IntoIterator<Item = &'c Type>, output: &'c ReturnType) {
        inputs.into_iter().for_each(|ty| self.crawl(ty));
        match output {
            ReturnType::Type(_, ty) => self.crawl(ty),
            ReturnType::Default => {}
        }
    }

    fn crawl_bound(&mut self, bound: &TypeParamBound) {
        match bound {
            TypeParamBound::Trait(ty) => self.crawl_path(None, &ty.path),
            TypeParamBound::Lifetime(lt) => self.try_collect(lt),
            _ => {}
        }
    }

    fn try_collect<'c>(&mut self, name: impl Into<GenericName<'c>>) {
        if let Some(info) = self.generics.get(name) {
            self.collection.push(info);
        }
    }
}
