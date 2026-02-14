use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Error, Expr, ExprLit, Field, Lit, Meta, Path, Result, Token};

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

pub struct FieldInfo<'a> {
    pub field: &'a Field,
    pub name: String,
    pub skip: bool,
    pub flatten: bool,
    pub allow_default: DefValue,
    pub into: Option<Path>,
    pub try_from: Option<Path>,
    pub deprecated: Option<String>,
    pub validate: Option<Path>,
}

impl<'a> FieldInfo<'a> {
    pub fn to_dynamic(&self) -> TokenStream {
        let name = &self.name;
        let ident = &self.field.ident;
        if self.skip {
            quote!()
        } else if self.flatten {
            quote!(
                self.#ident.place_dynamic(place);
            )
        } else if let Some(into) = &self.into {
            quote!(
                let target : #into = (&self.#ident).into();
                place.insert(#name.to_dynamic(), target.to_dynamic());
            )
        } else {
            quote!(
                place.insert(#name.to_dynamic(), self.#ident.to_dynamic());
            )
        }
    }

    pub fn from_dynamic(&self, struct_name: &str) -> TokenStream {
        let name = &self.name;
        let ident = &self.field.ident;
        let ty = &self.field.ty;

        let check_deprecated = if let Some(reason) = &self.deprecated {
            quote!(
                phaedra_dynamic::Error::raise_deprecated_fields(options, #struct_name, #name, #reason)?;
            )
        } else {
            quote!()
        };
        let validate_value = if let Some(validator) = &self.validate {
            quote!(
                #validator(&value).map_err(|msg| {
                    phaedra_dynamic::Error::ErrorInField{
                        type_name: #struct_name,
                        field_name: #name,
                        error: msg,
                    }
                })?;
            )
        } else {
            quote!()
        };

        if self.skip {
            quote!()
        } else if self.flatten {
            quote!(
                #ident:
                    <#ty>::from_dynamic(value, options)
                            .map_err(|source| source.field_context(
                                #struct_name,
                                #name,
                                obj))?,
            )
        } else if let Some(try_from) = &self.try_from {
            match &self.allow_default {
                DefValue::Default => {
                    quote!(
                        #ident: match obj.get_by_str(#name) {
                            Some(v) => {
                                use core::convert::TryFrom;
                                #check_deprecated
                                let target = <#try_from>::from_dynamic(v, options)
                                    .map_err(|source| source.field_context(
                                        #struct_name,
                                        #name,
                                        obj,
                                    ))?;
                                let value = <#ty>::try_from(target)
                                    .map_err(|source| phaedra_dynamic::Error::ErrorInField{
                                        type_name:#struct_name,
                                        field_name:#name,
                                        error: format!("{:#}", source)
                                    })?;
                                #validate_value
                                value
                            }
                            None => {
                                <#ty>::default()
                            }
                        },
                    )
                }
                DefValue::Path(default) => {
                    quote!(
                        #ident: match obj.get_by_str(&#name) {
                            Some(v) => {
                                use core::convert::TryFrom;
                                #check_deprecated
                                let target = <#try_from>::from_dynamic(v, options)
                                    .map_err(|source| source.field_context(
                                        #struct_name,
                                        #name,
                                        obj,
                                    ))?;
                                let value = <#ty>::try_from(target)
                                    .map_err(|source| phaedra_dynamic::Error::ErrorInField{
                                        type_name:#struct_name,
                                        field_name:#name,
                                        error: format!("{:#}", source),
                                    })?;
                                #validate_value
                                value
                            }
                            None => {
                                #default()
                            }
                        },
                    )
                }
                DefValue::None => {
                    quote!(
                        #ident: {
                            use core::convert::TryFrom;
                            let target = <#try_from>::from_dynamic(obj.get_by_str(#name).map(|v| {
                                #check_deprecated
                                v
                            }).unwrap_or(&Value::Null), options)
                                    .map_err(|source| source.field_context(
                                        #struct_name,
                                        #name,
                                        obj,
                                    ))?;
                            let value = <#ty>::try_from(target)
                                    .map_err(|source| phaedra_dynamic::Error::ErrorInField{
                                        type_name:#struct_name,
                                        field_name:#name,
                                        error: format!("{:#}", source),
                                    })?;
                            #validate_value
                            value
                        },
                    )
                }
            }
        } else {
            match &self.allow_default {
                DefValue::Default => {
                    quote!(
                        #ident: match obj.get_by_str(#name) {
                            Some(v) => {
                                #check_deprecated
                                let value = <#ty>::from_dynamic(v, options)
                                    .map_err(|source| source.field_context(
                                        #struct_name,
                                        #name,
                                        obj,
                                    ))?;
                                #validate_value
                                value
                            }
                            None => {
                                <#ty>::default()
                            }
                        },
                    )
                }
                DefValue::Path(default) => {
                    quote!(
                        #ident: match obj.get_by_str(#name) {
                            Some(v) => {
                                #check_deprecated
                                let value = <#ty>::from_dynamic(v, options)
                                    .map_err(|source| source.field_context(
                                        #struct_name,
                                        #name,
                                        obj,
                                    ))?;
                                #validate_value
                                value
                            }
                            None => {
                                #default()
                            }
                        },
                    )
                }
                DefValue::None => {
                    quote!(
                        #ident: {
                            let value = <#ty>::from_dynamic(
                                    obj.get_by_str(#name).map(|v| {
                                        #check_deprecated
                                        v
                                    }).
                                    unwrap_or(&Value::Null),
                                    options
                                )
                                .map_err(|source| source.field_context(#struct_name, #name, obj))?;
                            #validate_value
                            value
                        },
                    )
                }
            }
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

    for attr in &field.attrs {
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
            other => return Err(Error::new_spanned(other, "unsupported attribute")),
        }
    }

    Ok(FieldInfo {
        field,
        name,
        skip,
        flatten,
        allow_default,
        try_from,
        into,
        deprecated,
        validate,
    })
}
