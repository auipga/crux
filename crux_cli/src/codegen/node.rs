use std::hash::{Hash, Hasher};

use rustdoc_types::{
    Enum, ExternalCrate, GenericArg, GenericArgs, Id, Impl, Item, ItemEnum, ItemSummary, Path,
    Struct, StructKind, Type, Variant, VariantKind,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemNode(pub Item);

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct SummaryNode {
    pub id: Id,
    pub summary: ItemSummary,
}

impl Hash for SummaryNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.summary.path.hash(state);
    }
}

impl PartialEq for SummaryNode {
    fn eq(&self, other: &Self) -> bool {
        self.summary.path == other.summary.path
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct CrateNode {
    pub id: u32,
    pub crate_: ExternalCrate,
}

pub fn collect<'a, N: 'a, T: Iterator<Item = (&'a N,)>>(
    input: T,
) -> impl Iterator<Item = Vec<(&'a N,)>>
where
    N: Clone,
{
    std::iter::once(input.collect::<Vec<_>>())
}

impl SummaryNode {
    pub fn in_same_module_as(&self, other: &SummaryNode) -> bool {
        let this = &self.summary.path;
        let other = &other.summary.path;

        if this.len() != other.len() {
            return false;
        };

        this[..(this.len() - 1)] == other[..(other.len() - 1)]
    }
}

impl Hash for ItemNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let crate_id = self.0.crate_id;
        (crate_id, self.0.id).hash(state);
    }
}

impl ItemNode {
    pub fn name(&self) -> Option<&str> {
        let mut new_name = "";
        for attr in &self.0.attrs {
            if let Some((_, n)) =
                lazy_regex::regex_captures!(r#"\[serde\(rename\s*=\s*"(\w+)"\)\]"#, attr)
            {
                new_name = n;
            }
        }
        if new_name.is_empty() {
            self.0.name.as_deref()
        } else {
            Some(new_name)
        }
    }

    pub fn has_summary(&self, summary: &SummaryNode) -> bool {
        self.0.id == summary.id
    }

    pub fn is_struct(&self) -> bool {
        matches!(
            &self.0,
            Item {
                inner: ItemEnum::Struct(_),
                ..
            }
        )
    }

    pub fn is_struct_unit(&self) -> bool {
        matches!(
            &self.0,
            Item {
                inner: ItemEnum::Struct(Struct {
                    kind: StructKind::Unit,
                    ..
                }),
                ..
            }
        )
    }

    pub fn is_struct_plain(&self) -> bool {
        matches!(
            &self.0,
            Item {
                inner: ItemEnum::Struct(Struct {
                    kind: StructKind::Plain { .. },
                    ..
                }),
                ..
            }
        )
    }

    pub fn is_struct_tuple(&self) -> bool {
        matches!(
            &self.0,
            Item {
                inner: ItemEnum::Struct(Struct {
                    kind: StructKind::Tuple(_),
                    ..
                }),
                ..
            }
        )
    }

    pub fn is_enum(&self) -> bool {
        matches!(
            &self.0,
            Item {
                inner: ItemEnum::Enum(_),
                ..
            }
        )
    }

    pub fn is_impl_for(&self, for_: &ItemNode, trait_name: &str) -> bool {
        match &self.0 {
            Item {
                inner:
                    ItemEnum::Impl(Impl {
                        trait_: Some(Path { name, .. }),
                        for_: Type::ResolvedPath(Path { id, .. }),
                        ..
                    }),
                ..
            } if name == trait_name && id == &for_.0.id => true,
            _ => false,
        }
    }

    fn should_skip(&self) -> bool {
        self.0
            .attrs
            .iter()
            .any(|attr| lazy_regex::regex_is_match!(r#"\[serde\s*\(\s*skip\s*\)\s*\]"#, attr))
    }

    pub fn fields(&self, fields: Vec<(&ItemNode,)>) -> Vec<ItemNode> {
        let field_ids = match &self.0 {
            Item {
                inner: ItemEnum::Struct(Struct { kind, .. }),
                ..
            } => match kind {
                StructKind::Plain { fields, .. } => fields.to_vec(),
                StructKind::Tuple(fields) => {
                    fields.iter().filter_map(|f| f.as_ref()).cloned().collect()
                }
                StructKind::Unit => vec![],
            },
            Item {
                inner: ItemEnum::Variant(Variant { kind, .. }),
                ..
            } => match kind {
                VariantKind::Plain => vec![],
                VariantKind::Tuple(fields) => {
                    fields.iter().filter_map(|f| f.as_ref()).cloned().collect()
                }
                VariantKind::Struct { fields, .. } => fields.to_vec(),
            },
            _ => vec![],
        };
        field_ids
            .iter()
            .filter_map(
                |id| match fields.iter().find(|(f,)| !f.should_skip() && f.0.id == *id) {
                    Some(found) => Some(found.0.clone()),
                    None => None,
                },
            )
            .collect()
    }

    pub fn has_field(&self, field: &ItemNode) -> bool {
        if field.should_skip() {
            return false;
        }

        match &self.0 {
            Item {
                inner: ItemEnum::Struct(Struct { kind, .. }),
                ..
            } => {
                if field.name() == Some("__private_field") {
                    return false;
                }
                match kind {
                    StructKind::Unit => false,
                    StructKind::Tuple(fields) => fields.contains(&Some(field.0.id)),
                    StructKind::Plain {
                        fields,
                        has_stripped_fields: _,
                    } => fields.contains(&field.0.id),
                }
            }
            Item {
                inner: ItemEnum::Variant(Variant { kind, .. }),
                ..
            } => match kind {
                VariantKind::Plain => false,
                VariantKind::Tuple(fields) => fields.contains(&Some(field.0.id)),
                VariantKind::Struct { fields, .. } => fields.contains(&field.0.id),
            },
            _ => false,
        }
    }

    pub fn variants(&self, variants: Vec<(&ItemNode,)>) -> Vec<ItemNode> {
        let variant_ids = match &self.0 {
            Item {
                inner: ItemEnum::Enum(Enum { variants, .. }),
                ..
            } => variants.to_vec(),
            _ => vec![],
        };
        variant_ids
            .iter()
            .filter_map(|id| {
                match variants
                    .iter()
                    .find(|(v,)| !v.should_skip() && v.0.id == *id)
                {
                    Some(found) => Some(found.0.clone()),
                    None => None,
                }
            })
            .collect()
    }

    pub fn has_variant(&self, variant: &ItemNode) -> bool {
        if variant.should_skip() {
            return false;
        }

        match &self.0 {
            Item {
                inner: ItemEnum::Enum(Enum { variants, .. }),
                ..
            } => variants.contains(&variant.0.id),
            _ => false,
        }
    }

    pub fn is_of_local_type(&self, type_node: &ItemNode) -> bool {
        self.is_of_type(&type_node.0.id, false)
    }

    pub fn is_of_remote_type(&self, type_node: &SummaryNode) -> bool {
        self.is_of_type(&type_node.id, true)
    }

    fn is_of_type(&self, id: &Id, is_remote: bool) -> bool {
        match &self.0 {
            Item {
                inner: ItemEnum::StructField(t),
                ..
            } => check_type(&id, t, is_remote),
            Item {
                inner:
                    ItemEnum::AssocType {
                        type_: Some(Type::ResolvedPath(target)),
                        ..
                    },
                ..
            } => &target.id == id,
            _ => false,
        }
    }

    pub fn has_associated_item(&self, associated_item: &ItemNode, with_name: &str) -> bool {
        match &self.0 {
            Item {
                inner: ItemEnum::Impl(Impl { items, .. }),
                ..
            } => match &associated_item.0 {
                Item {
                    name: Some(name), ..
                } if with_name == name => items.contains(&associated_item.0.id),
                _ => false,
            },
            _ => false,
        }
    }
}

fn check_type(parent: &Id, type_: &Type, is_remote: bool) -> bool {
    match type_ {
        Type::ResolvedPath(Path { name, id, args }) => {
            if is_remote {
                if let "Option" | "String" | "Vec" = name.as_str() {
                    return false;
                }
            }
            id == parent || {
                if let Some(args) = args {
                    check_args(parent, args, is_remote)
                } else {
                    false
                }
            }
        }
        Type::QualifiedPath {
            self_type, args, ..
        } => {
            if is_remote {
                // record types in foreign crates, for continuation
                check_type(parent, self_type, is_remote) || check_args(parent, args, is_remote)
            } else {
                false
            }
        }
        Type::Primitive(_) => false,
        Type::Tuple(vec) => vec.iter().any(|t| check_type(parent, t, is_remote)),
        Type::Slice(t) => check_type(parent, t, is_remote),
        Type::Array { type_: t, .. } => check_type(parent, t, is_remote),
        _ => false,
    }
}

fn check_args(parent: &Id, args: &Box<GenericArgs>, is_remote: bool) -> bool {
    match args.as_ref() {
        GenericArgs::AngleBracketed { args, .. } => args.iter().any(|arg| match arg {
            GenericArg::Type(t) => check_type(parent, t, is_remote),
            _ => false,
        }),
        GenericArgs::Parenthesized { inputs, .. } => {
            inputs.iter().any(|t| check_type(parent, t, is_remote))
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rustdoc_types::{Generics, Id, Item, ItemEnum, ItemKind, Struct, StructKind, Visibility};

    use super::*;

    fn make_summary(id: Id, path: Vec<String>) -> SummaryNode {
        SummaryNode {
            id,
            summary: ItemSummary {
                crate_id: 0,
                path,
                kind: ItemKind::Struct,
            },
        }
    }

    fn make_node(name: Option<String>, attrs: Vec<String>) -> ItemNode {
        ItemNode(Item {
            name,
            attrs,
            inner: ItemEnum::Struct(Struct {
                kind: StructKind::Plain {
                    fields: vec![],
                    has_stripped_fields: false,
                },
                generics: Generics {
                    params: vec![],
                    where_predicates: vec![],
                },
                impls: vec![],
            }),
            id: Id(0),
            crate_id: 0,
            span: None,
            visibility: Visibility::Public,
            docs: None,
            links: Default::default(),
            deprecation: None,
        })
    }

    #[test]
    fn test_in_same_module_as() {
        let summary1 = make_summary(Id(0), vec!["foo".to_string(), "bar".to_string()]);
        let summary2 = make_summary(Id(1), vec!["foo".to_string(), "baz".to_string()]);
        assert!(summary1.in_same_module_as(&summary2));
    }

    #[test]
    fn test_in_same_module_as_different_length() {
        let summary1 = make_summary(Id(0), vec!["foo".to_string(), "bar".to_string()]);
        let summary2 = make_summary(Id(1), vec!["foo".to_string()]);
        assert!(!summary1.in_same_module_as(&summary2));
    }

    #[test]
    fn test_in_same_module_as_different_module() {
        let summary1 = make_summary(Id(0), vec!["foo".to_string(), "bar".to_string()]);
        let summary2 = make_summary(Id(1), vec!["baz".to_string(), "bar".to_string()]);
        assert!(!summary1.in_same_module_as(&summary2));
    }

    #[test]
    fn test_get_name() {
        let name = Some("Foo".to_string());
        let attrs = vec![];
        let node = make_node(name, attrs);
        assert_eq!(node.name(), Some("Foo"));
    }

    #[test]
    fn test_get_name_with_rename() {
        let name = Some("Foo".to_string());
        let attrs = vec![r#"#[serde(rename = "Bar")]"#.to_string()];
        let node = make_node(name, attrs);
        assert_eq!(node.name(), Some("Bar"));
    }

    #[test]
    fn test_get_name_with_rename_no_whitespace() {
        let name = Some("Foo".to_string());
        let attrs = vec![r#"#[serde(rename="Bar")]"#.to_string()];
        let node = make_node(name, attrs);
        assert_eq!(node.name(), Some("Bar"));
    }

    #[test]
    fn test_get_name_with_no_name() {
        let name = None;
        let attrs = vec![];
        let node = make_node(name, attrs);
        assert_eq!(node.name(), None);
    }
}
