use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Error, Expr, ExprLit, Field, GenericArgument, Ident, Lit, Meta, Path,
    PathArguments, Result, Token, Type,
};

fn parse_string(expr: &Expr) -> Result<String> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => Ok(s.value()),
        _ => Err(Error::new_spanned(expr, "expected string literal")),
    }
}

fn parse_path(expr: &Expr) -> Result<Path> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => s.parse(),
        Expr::Path(path) => Ok(path.path.clone()),
        _ => Err(Error::new_spanned(expr, "expected string literal or path")),
    }
}

#[allow(unused)]
pub struct ContainerInfo {
    pub into: Option<Path>,
    pub try_from: Option<Path>,
    pub debug: bool,
}

pub fn container_info(attrs: &[Attribute]) -> Result<ContainerInfo> {
    let mut into = None;
    let mut try_from = None;
    let mut debug = false;

    for attr in attrs {
        if !attr.path().is_ident("dynamic") {
            continue;
        }

        match &attr.meta {
            Meta::List(_) => {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("debug") {
                        if meta.input.peek(Token![=]) {
                            return Err(meta.error("unsupported attribute"));
                        }
                        debug = true;
                        return Ok(());
                    }

                    if meta.path.is_ident("into") {
                        let expr: Expr = meta.value()?.parse()?;
                        into = Some(parse_path(&expr)?);
                        return Ok(());
                    }

                    if meta.path.is_ident("try_from") {
                        let expr: Expr = meta.value()?.parse()?;
                        try_from = Some(parse_path(&expr)?);
                        return Ok(());
                    }

                    Err(meta.error("unsupported attribute"))
                })?;
            }
            other => return Err(Error::new_spanned(other, "unsupported attribute")),
        }
    }

    Ok(ContainerInfo {
        into,
        try_from,
        debug,
    })
}

pub enum DefValue {
    None,
    Default,
    Path(Path),
}

#[allow(unused)]
pub struct FieldInfo<'a> {
    pub field: &'a Field,
    pub type_name: String,
    pub name: String,
    pub skip: bool,
    pub flatten: bool,
    pub allow_default: DefValue,
    pub into: Option<Path>,
    pub try_from: Option<Path>,
    pub deprecated: Option<String>,
    pub validate: Option<Path>,
    pub doc: String,
    pub container_type: ContainerType,
}

#[derive(Debug)]
pub enum ContainerType {
    None,
    Option,
    Vec,
    Map,
}

impl<'a> FieldInfo<'a> {
    pub fn to_option(&self) -> TokenStream {
        let name = &self.name;
        let doc = &self.doc;
        let type_name = &self.type_name;
        let container_type = Ident::new(&format!("{:?}", self.container_type), Span::call_site());
        let get_default = match self.compute_default() {
            Some(def) => quote!(Some(|| #def.to_dynamic())),
            None => quote!(None),
        };
        quote!(
            crate::meta::ConfigOption {
                name: #name,
                doc: #doc,
                tags: &[],
                container: crate::meta::ConfigContainer::#container_type,
                type_name: #type_name,
                default_value: #get_default,
                possible_values: &[],
                fields: &[],
            }
        )
    }

    fn compute_default(&self) -> Option<TokenStream> {
        let ty = &self.field.ty;
        match &self.allow_default {
            DefValue::Default => Some(quote!(
                <#ty>::default()
            )),
            DefValue::Path(default) => Some(quote!(
                #default()
            )),
            DefValue::None => None,
        }
    }
}

pub fn field_info(field: &Field) -> Result<FieldInfo<'_>> {
    let mut name = field.ident.as_ref().unwrap().to_string();
    let mut skip = false;
    let mut flatten = false;
    let mut allow_default = DefValue::None;
    let mut try_from = None;
    let mut validate = None;
    let mut into = None;
    let mut deprecated = None;
    let mut doc = String::new();
    let mut container_type = ContainerType::None;

    let type_name = match &field.ty {
        Type::Path(p) => {
            let last_seg = p.path.segments.last().unwrap();
            match &last_seg.arguments {
                PathArguments::None => last_seg.ident.to_string(),
                PathArguments::AngleBracketed(args) if args.args.len() == 1 => {
                    let arg = args.args.first().unwrap();
                    match arg {
                        GenericArgument::Type(Type::Path(t)) => {
                            container_type = match last_seg.ident.to_string().as_str() {
                                "Option" => ContainerType::Option,
                                "Vec" => ContainerType::Vec,
                                _ => panic!("unhandled type for {name}: {:#?}", field.ty),
                            };
                            t.path.segments.last().unwrap().ident.to_string()
                        }
                        _ => panic!("unhandled type for {name}: {:#?}", field.ty),
                    }
                }
                PathArguments::AngleBracketed(args) if args.args.len() == 2 => {
                    let arg = args.args.last().unwrap();
                    match arg {
                        GenericArgument::Type(Type::Path(t)) => {
                            container_type = match last_seg.ident.to_string().as_str() {
                                "HashMap" => ContainerType::Map,
                                _ => panic!("unhandled type for {name}: {:#?}", field.ty),
                            };
                            t.path.segments.last().unwrap().ident.to_string()
                        }
                        _ => panic!("unhandled type for {name}: {:#?}", field.ty),
                    }
                }
                _ => panic!("unhandled type for {name}: {:#?}", field.ty),
            }
        }
        _ => panic!("unhandled type for {name}: {:#?}", field.ty),
    };

    for attr in &field.attrs {
        if attr.path().is_ident("doc") {
            match &attr.meta {
                Meta::NameValue(value) => {
                    let part = parse_string(&value.value)?;
                    if !doc.is_empty() {
                        doc.push('\n');
                    }
                    doc.push_str(&part);
                }
                other => {
                    return Err(Error::new_spanned(
                        other,
                        format!("unsupported attribute {other:?}"),
                    ))
                }
            }
            continue;
        }

        if !attr.path().is_ident("dynamic") {
            continue;
        }

        match &attr.meta {
            Meta::List(_) => {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let expr: Expr = meta.value()?.parse()?;
                        name = parse_string(&expr)?;
                        return Ok(());
                    }

                    if meta.path.is_ident("default") {
                        if meta.input.peek(Token![=]) {
                            let expr: Expr = meta.value()?.parse()?;
                            allow_default = DefValue::Path(parse_path(&expr)?);
                        } else {
                            allow_default = DefValue::Default;
                        }
                        return Ok(());
                    }

                    if meta.path.is_ident("deprecated") {
                        let expr: Expr = meta.value()?.parse()?;
                        deprecated.replace(parse_string(&expr)?);
                        return Ok(());
                    }

                    if meta.path.is_ident("into") {
                        let expr: Expr = meta.value()?.parse()?;
                        into = Some(parse_path(&expr)?);
                        return Ok(());
                    }

                    if meta.path.is_ident("try_from") {
                        let expr: Expr = meta.value()?.parse()?;
                        try_from = Some(parse_path(&expr)?);
                        return Ok(());
                    }

                    if meta.path.is_ident("validate") {
                        let expr: Expr = meta.value()?.parse()?;
                        validate = Some(parse_path(&expr)?);
                        return Ok(());
                    }

                    if meta.path.is_ident("skip") {
                        if meta.input.peek(Token![=]) {
                            return Err(meta.error("unsupported attribute"));
                        }
                        skip = true;
                        return Ok(());
                    }

                    if meta.path.is_ident("flatten") {
                        if meta.input.peek(Token![=]) {
                            return Err(meta.error("unsupported attribute"));
                        }
                        flatten = true;
                        return Ok(());
                    }

                    Err(meta.error("unsupported attribute"))
                })?;
            }
            other => {
                return Err(Error::new_spanned(
                    other,
                    format!("unsupported attribute {other:?}"),
                ))
            }
        }
    }

    Ok(FieldInfo {
        type_name,
        field,
        name,
        skip,
        flatten,
        allow_default,
        try_from,
        into,
        deprecated,
        validate,
        doc,
        container_type,
    })
}
