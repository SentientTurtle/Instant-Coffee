#![feature(try_blocks)]
#![feature(proc_macro_span)]
#![feature(iter_collect_into)]

extern crate proc_macro;

use proc_macro::{TokenStream};
use std::collections::{HashMap, HashSet};
use quote::{format_ident, quote, ToTokens};
use syn::{Attribute, Field, Fields, FnArg, Ident, ImplItem, ImplItemFn, Item, ItemEnum, ItemFn, ItemMod, ItemStruct, Lit, LitInt, LitStr, Meta, parse_quote, Pat, Path, PathArguments, ReturnType, Signature, Token, Type, TypePath, TypeTuple, Visibility};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Paren;

/// Verify that the given string is a valid java identifier
fn verify_java_identifier(identifier: &str) -> Result<(), String> {
    if identifier.len() == 0 {
        return Err("Java identifiers must be at least 1 character long".to_string());
    }

    let first_char_valid = identifier.chars()
        .next()
        .is_some_and(|char| {
            // TODO: Feature for broader unicode support & accurately matching java's rules
            match char {
                'A'..='Z' => true,
                'a'..='z' => true,
                '_' => true,
                '$' => true,
                _ => false
            }
        });
    if !first_char_valid {
        return Err("Java identifiers may only start with `A-Z`, `a-z`, `_` or `$`".to_string());
    }

    let all_chars_valid = identifier.chars().all(|char| match char {
        'A'..='Z' => true,
        'a'..='z' => true,
        '0'..='9' => true,
        '_' => true,
        '$' => true,
        _ => false
    });
    if !all_chars_valid {
        return Err("Java identifiers may only contain `A-Z`, `a-z`, `0-9`, `_` or `$`".to_string());
    }

    let name_is_keyword = match identifier {
        "abstract" => true,
        "assert" => true,
        "boolean" => true,
        "break" => true,
        "byte" => true,
        "case" => true,
        "catch" => true,
        "char" => true,
        "class" => true,
        "const" => true,
        "continue" => true,
        "default" => true,
        "do" => true,
        "double" => true,
        "else" => true,
        "enum" => true,
        "extends" => true,
        "final" => true,
        "finally" => true,
        "float" => true,
        "for" => true,
        "if" => true,
        "goto" => true,
        "implements" => true,
        "import" => true,
        "instanceof" => true,
        "int" => true,
        "interface" => true,
        "long" => true,
        "native" => true,
        "new" => true,
        "package" => true,
        "private" => true,
        "protected" => true,
        "public" => true,
        "return" => true,
        "short" => true,
        "static" => true,
        "strictfp" => true,
        "super" => true,
        "switch" => true,
        "synchronized" => true,
        "this" => true,
        "throw" => true,
        "throws" => true,
        "transient" => true,
        "try" => true,
        "void" => true,
        "volatile" => true,
        "while" => true,
        "_" => true,
        "true" => true,
        "false" => true,
        "null" => true,
        _ => false
    };

    if name_is_keyword {
        return Err(format!("Java identifiers may not be keyword `{}`", identifier));
    }
    Ok(())
}

/// Verify that the given string is a valid java type identifier (classname)
fn verify_type_identifier(identifier: &str) -> Result<(), String> {
    verify_java_identifier(identifier)?;
    let is_reserved = match identifier {
        "permits" => true,
        "record" => true,
        "sealed" => true,
        "var" => true,
        "yield" => true,
        _ => false
    };

    if is_reserved {
        return Err(format!("Java identifiers may not be keyword `{}`", identifier));
    }
    Ok(())
}

/// Verify that the given string is a valid java package identifier (qualified 'path')
fn verify_package_identifier(decl: &str) -> Result<(), String> {
    for name in decl.split('.') {
        verify_java_identifier(name)?;
    }
    Ok(())
}

enum ClassKind {
    /// Rust struct
    Struct(ItemStruct),
    /// Rust enum
    Enum(ItemEnum),
}

impl ClassKind {
    /// delegate to macro implementations
    fn generate(self) -> Result<TokenStream, syn::Error> {
        match self {
            ClassKind::Struct(item_struct) => impl_struct_gen(item_struct),
            ClassKind::Enum(item_enum) => impl_enum_gen(item_enum),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum StructKind {
    Named,
    Tuple,
    Unit,
}

/// Reads info macros for a given Ident, expects [`jmodule_package`] and [`jmodule_methods`]
///
/// # Arguments
///
/// * `ident_span`: Span to attach errors to; Should be the Ident of the struct attributes are attached to
/// * `attributes`: Attribute macros to scan
///
/// returns: If Ok, (Package path, methods)
fn read_jmodule_info(ident_span: proc_macro2::Span, attributes: Vec<Attribute>) -> Result<(String, Vec<Signature>), syn::Error> {
    let mut package_name = None;
    let mut method_list = None;
    for attribute in attributes {
        if let Meta::List(ref list) = attribute.meta {
            if list.path.segments.last().is_some_and(|segment| segment.ident == "jmodule_package") {
                if package_name.is_some() {
                    Err(syn::Error::new(attribute.span(), "duplicate jmodule_package"))?;
                }

                let package_literal = syn::parse::<LitStr>(list.tokens.to_token_stream().into())?;
                let name = package_literal.value();
                verify_package_identifier(&name).map_err(|e| syn::Error::new(package_literal.span(), e))?;

                package_name = Some(name)
            } else if list.path.segments.last().is_some_and(|segment| segment.ident == "jmodule_methods") {
                if method_list.is_some() {
                    Err(syn::Error::new(attribute.span(), "duplicate jmodule_methods"))?;
                }

                let signatures = Punctuated::<Signature, Token![,]>::parse_terminated.parse(list.tokens.to_token_stream().into())?;

                method_list = Some(signatures.into_iter().collect::<Vec<_>>());
            }
        }
    }

    if let (Some(package), Some(methods)) = (package_name, method_list) {
        Ok((package, methods))
    } else {
        Err(syn::Error::new(ident_span.into(), "Missing jmodule context!"))
    }
}

/// Turn syn function signatures into `JMethod` declarations
fn quote_method_decls(signatures: Vec<Signature>) -> Result<Vec<proc_macro2::TokenStream>, syn::Error> {
    let mut method_decls = Vec::new();
    for signature in signatures {
        let method_name = signature.ident.to_string();
        verify_java_identifier(&method_name).map_err(|e| syn::Error::new(signature.ident.span(), e))?;

        let mut is_static = true;
        let inputs = signature.inputs.into_iter().flat_map(|input| {
            match input {
                FnArg::Receiver(_) => {
                    is_static = false;
                    None
                }
                FnArg::Typed(input_type) => {
                    let param_name = match *input_type.pat {
                        Pat::Ident(ident) => ident.ident.to_string(),
                        _ => unreachable!("invalid jmodule_methods macro")
                    };

                    let i_ty = *input_type.ty;
                    Some(quote!((#param_name, <#i_ty as instant_coffee::JavaType>::QUALIFIED_NAME())))
                }
            }
        }).collect::<Vec<_>>();
        let o_ty = match signature.output {
            ReturnType::Default => parse_quote!(()),
            ReturnType::Type(_, return_type) => *return_type
        };
        let output = quote!(<#o_ty as instant_coffee::JavaReturn>::QUALIFIED_NAME());

        method_decls.push(
            quote!(instant_coffee::codegen::JMethod {
                is_static: #is_static,
                name: #method_name,
                inputs: vec![#(#inputs),*],
                output: #output
            })
        );
    }

    Ok(method_decls)
}

// Turn syn fields into `JField` declarations
fn quote_fields<T: IntoIterator<Item=Field>>(fields: T) -> Result<(Vec<Ident>, Vec<proc_macro2::TokenStream>, Vec<Type>, Vec<proc_macro2::TokenStream>), syn::Error> {
    let mut field_names = Vec::new();
    let mut field_idents = Vec::new();
    let mut field_types = Vec::new();
    let mut field_decls = Vec::new();
    for (idx, field) in fields.into_iter().enumerate() {
        let r_ty = field.ty;
        let j_ty = quote!(<#r_ty as instant_coffee::JavaType>::QUALIFIED_NAME());
        let vis = match field.vis {
            Visibility::Public(_) => quote!(instant_coffee::codegen::JAccessModifier::Public),
            Visibility::Inherited => quote!(instant_coffee::codegen::JAccessModifier::Private),
            vis @ Visibility::Restricted(_) => Err(syn::Error::new(vis.span(), "only pub or private visibility is supported"))?,
        };
        let name_string = field.ident.as_ref().map(Ident::to_string).unwrap_or(format!("field_{}", idx));

        // Tuple 'fields' are accessed by integer literal, not an ident token
        field_idents.push(
            field.ident.as_ref()
                .map(|ident| ident.to_token_stream())
                .unwrap_or(LitInt::new(&*idx.to_string(), proc_macro2::Span::call_site()).to_token_stream())
        );

        let name_ident = field.ident.map(|ident| format_ident!("{}", ident)).unwrap_or_else(|| format_ident!("field_{}", idx));
        verify_java_identifier(&name_string).map_err(|e| syn::Error::new(name_ident.span(), e))?;

        field_names.push(name_ident);
        field_types.push(r_ty.clone());
        field_decls.push(quote!(instant_coffee::codegen::JField { access: #vis, jtype: #j_ty, name: #name_string }));
    }
    Ok((
        field_names,
        field_idents,
        field_types,
        field_decls,
    ))
}

fn impl_struct_gen(item_struct: ItemStruct) -> Result<TokenStream, syn::Error> {
    let struct_kind = match &item_struct.fields {
        Fields::Named(_) => StructKind::Named,
        Fields::Unnamed(_) => StructKind::Tuple,
        Fields::Unit => StructKind::Unit
    };

    let (package_name_str, method_signatures) = read_jmodule_info(item_struct.ident.span(), item_struct.attrs)?;    // read jmodule info verifies that the package name is a valid java name
    let struct_name_str = item_struct.ident.to_string();
    let name_ident = item_struct.ident;
    let qualified_name_str = format!("{}.{}", package_name_str, struct_name_str);
    let jvm_class_name_str = format!("{}/{}", package_name_str.replace('.', "/"), struct_name_str);
    let jvm_param_sig_str = format!("L{}/{};", package_name_str.replace('.', "/"), struct_name_str);
    let (impl_generics, type_generics, where_clause) = item_struct.generics.split_for_impl();
    let method_decls = quote_method_decls(method_signatures)?;   // quote method decls verifies method names are valid java names

    verify_type_identifier(&struct_name_str).map_err(|e| syn::Error::new(name_ident.span(), e))?;

    let (
        field_names,
        field_idents,
        field_types,
        field_decls,
    ) = quote_fields(item_struct.fields)?;  // quote fields verifies that field names are valid java names

    let from_jni_impl = match struct_kind {
        StructKind::Named => quote! {
            fn from_jni<'local>(jni_value: jni::objects::JObject<'local>, env: &mut jni::JNIEnv<'local>) -> Result<Self, Option<jni::errors::Exception>> {
                Ok(Self {#(
                    #field_idents: <#field_types as instant_coffee::JavaType>::from_jni(
                        <#field_types as instant_coffee::JavaType>::from_jvalue(
                            env.get_field(&jni_value, stringify!(#field_names), <#field_types as instant_coffee::JavaType>::JVM_PARAM_SIGNATURE())
                                .map_err(instant_coffee::jni_util::map_jni_error)?,
                            env
                        )?,
                        env
                    )?
                ),*})
            }
        },
        StructKind::Tuple => quote! {
            fn from_jni<'local>(jni_value: jni::objects::JObject<'local>, env: &mut jni::JNIEnv<'local>) -> Result<Self, Option<jni::errors::Exception>> {
                Ok(Self (#(
                    <#field_types as instant_coffee::JavaType>::from_jni(
                        <#field_types as instant_coffee::JavaType>::from_jvalue(
                            env.get_field(&jni_value, stringify!(#field_names), <#field_types as instant_coffee::JavaType>::JVM_PARAM_SIGNATURE())
                                .map_err(instant_coffee::jni_util::map_jni_error)?,
                            env
                        )?,
                        env
                    )?
                ),*))
            }
        },
        StructKind::Unit => quote! {
            fn from_jni<'local>(jni_value: jni::objects::JObject<'local>, env: &mut jni::JNIEnv<'local>) -> Result<Self, Option<jni::errors::Exception>> {
                Ok(Self)
            }
        }
    };


    let exp = quote! {
        impl #impl_generics instant_coffee::codegen::JavaClass for #name_ident #type_generics #where_clause {
            fn declaration() -> instant_coffee::codegen::JClassDecl {
                instant_coffee::codegen::JClassDecl::Class {
                    name: #struct_name_str,
                    package: #package_name_str,
                    fields: vec![#(#field_decls),*],
                    methods: vec![#(#method_decls),*]
                }
            }
        }

        impl #impl_generics instant_coffee::JavaType for #name_ident #type_generics #where_clause {
            type JniType<'local> = jni::objects::JObject<'local>;
            type ArrayType<'local> = jni::objects::JObjectArray<'local>;

            fn QUALIFIED_NAME() -> &'static str { #qualified_name_str }

            fn JVM_PARAM_SIGNATURE() -> &'static str {#jvm_param_sig_str }

            fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { jni::objects::JObject::null() }

            fn from_jvalue<'local>(jvalue: jni::objects::JValueOwned<'local>, _env: &mut jni::JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<jni::errors::Exception>> {
                match jvalue {
                    jni::objects::JValueOwned::Object(obj) => Ok(obj),
                    _ => Err(Some(jni::errors::Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as instant_coffee::JavaType>::QUALIFIED_NAME()) }))
                }
            }

            fn into_jni<'local>(self, env: &mut jni::JNIEnv<'local>) -> Result<jni::objects::JObject<'local>, Option<jni::errors::Exception>> {
                #(let #field_names = jni::objects::JValueOwned::from(<#field_types as instant_coffee::JavaType>::into_jni(self.#field_idents, env)?);)*

                let args = &[
                    #(jni::objects::JValue::from(&#field_names)),*
                ];

                env.new_object(
                    #jvm_class_name_str,
                    [
                        "(",
                        #(<#field_types as instant_coffee::JavaType>::JVM_PARAM_SIGNATURE(),)*
                        ")V"
                    ].join(""), // Micro-optimization candidate: Use const-cat
                    args
                )
                .map_err(instant_coffee::jni_util::map_jni_error)
            }

            #from_jni_impl
        }
    };

    Ok(exp.into())
}

fn impl_enum_gen(item_enum: ItemEnum) -> Result<TokenStream, syn::Error> {
    let (package_name_str, method_signatures) = read_jmodule_info(item_enum.ident.span(), item_enum.attrs)?;
    let enum_name_str = item_enum.ident.to_string();
    let name_ident = item_enum.ident;
    let qualified_name_str = format!("{}.{}", package_name_str, enum_name_str);
    let jvm_class_name_str = format!("{}/{}", package_name_str.replace('.', "/"), enum_name_str);
    let jvm_param_sig_str = format!("L{}/{};", package_name_str.replace('.', "/"), enum_name_str);
    let (impl_generics, type_generics, where_clause) = item_enum.generics.split_for_impl();
    let method_decls = quote_method_decls(method_signatures)?;   // quote method decls verifies method names are valid java names

    verify_type_identifier(&enum_name_str).map_err(|e| syn::Error::new(name_ident.span(), e))?;

    let is_tagged_union = item_enum.variants.iter().any(|variant| variant.fields != Fields::Unit);

    let class_decl_impl;
    let into_jni_impl;
    let from_jni_impl;

    if is_tagged_union {
        let mut variant_decls = Vec::new();
        let mut variant_into_jni_expressions = Vec::new();
        let mut variant_from_jni_expressions = Vec::new();
        for variant in item_enum.variants {
            let variant_ident = variant.ident;
            let variant_name = variant_ident.to_string();
            let jvm_variant_name_str = format!("{}${}", jvm_class_name_str, variant_name);

            verify_type_identifier(&variant_name).map_err(|e| syn::Error::new(name_ident.span(), e))?;

            let variant_kind = match &variant.fields {
                Fields::Named(_) => StructKind::Named,
                Fields::Unnamed(_) => StructKind::Tuple,
                Fields::Unit => StructKind::Unit
            };

            let (
                field_names,
                field_idents,
                field_types,
                field_decls,
            ) = quote_fields(variant.fields)?;  // quote fields verifies that field names are valid java names

            variant_decls.push(quote! {
                instant_coffee::codegen::JUnionVariant {
                    name: #variant_name,
                    fields: vec![#(#field_decls),*]
                }
            });

            match variant_kind {
                StructKind::Named => {
                    variant_into_jni_expressions.push(quote! {
                        #name_ident::#variant_ident { #(#field_idents),* } => {
                            #(let #field_names = jni::objects::JValueOwned::from(<#field_types as instant_coffee::JavaType>::into_jni(#field_idents, env)?);)*

                            let args = &[
                                #(jni::objects::JValue::from(&#field_names)),*
                            ];

                            env.new_object(
                                #jvm_variant_name_str,
                                [
                                    "(",
                                    #(<#field_types as instant_coffee::JavaType>::JVM_PARAM_SIGNATURE(),)*
                                    ")V"
                                ].join(""), // Micro-optimization candidate: Use const-cat
                                args
                            )
                            .map_err(instant_coffee::jni_util::map_jni_error)
                        }
                    });

                    variant_from_jni_expressions.push(quote! {
                        if env.is_instance_of(&jni_value, #jvm_variant_name_str).map_err(instant_coffee::jni_util::map_jni_error)? {
                            return Ok(#name_ident::#variant_ident {#(
                                #field_idents: <#field_types as instant_coffee::JavaType>::from_jni(
                                    <#field_types as instant_coffee::JavaType>::from_jvalue(
                                        env.get_field(&jni_value, stringify!(#field_names), <#field_types as instant_coffee::JavaType>::JVM_PARAM_SIGNATURE())
                                            .map_err(instant_coffee::jni_util::map_jni_error)?,
                                        env
                                    )?,
                                    env
                                )?
                            ),*});
                        }
                    })
                },
                StructKind::Tuple => {
                    variant_into_jni_expressions.push(quote! {
                        #name_ident::#variant_ident ( #(#field_names),* ) => {
                            #(let #field_names = jni::objects::JValueOwned::from(<#field_types as instant_coffee::JavaType>::into_jni(#field_idents, env)?);)*

                            let args = &[
                                #(jni::objects::JValue::from(&#field_names)),*
                            ];

                            env.new_object(
                                #jvm_variant_name_str,
                                [
                                    "(",
                                    #(<#field_types as instant_coffee::JavaType>::JVM_PARAM_SIGNATURE(),)*
                                    ")V"
                                ].join(""), // Micro-optimization candidate: Use const-cat
                                args
                            )
                            .map_err(instant_coffee::jni_util::map_jni_error)
                        }
                    });

                    variant_from_jni_expressions.push(quote! {
                        if env.is_instance_of(&jni_value, #jvm_variant_name_str).map_err(instant_coffee::jni_util::map_jni_error)? {
                            return Ok(#name_ident::#variant_ident (#(
                                <#field_types as instant_coffee::JavaType>::from_jni(
                                    <#field_types as instant_coffee::JavaType>::from_jvalue(
                                        env.get_field(&jni_value, stringify!(#field_names), <#field_types as instant_coffee::JavaType>::JVM_PARAM_SIGNATURE())
                                            .map_err(instant_coffee::jni_util::map_jni_error)?,
                                        env
                                    )?,
                                    env
                                )?
                            ),*));
                        }
                    })
                },
                StructKind::Unit => {
                    variant_into_jni_expressions.push(quote! {
                        #name_ident::#variant_ident => {
                            env.new_object(#jvm_variant_name_str,"()V",&[]).map_err(instant_coffee::jni_util::map_jni_error)
                        }
                    });

                    variant_from_jni_expressions.push(quote! {
                        if env.is_instance_of(&jni_value, #jvm_variant_name_str).map_err(instant_coffee::jni_util::map_jni_error)? {
                            return Ok(#name_ident::#variant_ident);
                        }
                    })
                }
            }
        }

        class_decl_impl = quote! {
            fn declaration() -> instant_coffee::codegen::JClassDecl {
                instant_coffee::codegen::JClassDecl::EnumTaggedUnion {
                    name: #enum_name_str,
                    package: #package_name_str,
                    variants: vec![#(#variant_decls),*],
                    methods: vec![#(#method_decls),*]
                }
            }
        };

        into_jni_impl = quote! {
            fn into_jni<'local>(self, env: &mut jni::JNIEnv<'local>) -> Result<jni::objects::JObject<'local>, Option<jni::errors::Exception>> {
                match self {
                    #(#variant_into_jni_expressions)*
                }
            }
        };

        from_jni_impl = quote! {
            fn from_jni<'local>(jni_value: jni::objects::JObject<'local>, env: &mut jni::JNIEnv<'local>) -> Result<Self, Option<jni::errors::Exception>> {
                #(#variant_from_jni_expressions)*
                // If none of the above blocks match and return, somehow none of the variant subclasses match
                let class_name = instant_coffee::jni_util::obj_classname(&jni_value, env).unwrap_or("[UNKNOWN]".to_string());

                Err(Some(jni::errors::Exception { class: "java/lang/RuntimeException".to_string(), msg: format!("JNI: Could not match {} as Rust Enum: {}", #enum_name_str, class_name)}))
            }
        };
    } else {
        let mut variant_names = Vec::new();
        for variant in &item_enum.variants {
            let name = variant.ident.to_string();
            verify_type_identifier(&name).map_err(|e| syn::Error::new(variant.ident.span(), e))?;
            variant_names.push(name);
        }

        let variant_idents = item_enum.variants.into_iter().map(|variant| variant.ident).collect::<Vec<_>>();
        let ordinals = (0..variant_idents.len()).into_iter().map(|ord| LitInt::new(&format!("{}", ord), proc_macro2::Span::call_site())).collect::<Vec<_>>();

        class_decl_impl = quote! {
            fn declaration() -> instant_coffee::codegen::JClassDecl {
                instant_coffee::codegen::JClassDecl::Enum {
                    name: #enum_name_str,
                    package: #package_name_str,
                    variants: vec![#(#variant_names),*],
                    methods: vec![#(#method_decls),*]
                }
            }
        };

        into_jni_impl = quote! {
            fn into_jni<'local>(self, env: &mut jni::JNIEnv<'local>) -> Result<jni::objects::JObject<'local>, Option<jni::errors::Exception>> {
                match self {
                    #(#name_ident::#variant_idents => {
                        env.get_static_field(#jvm_class_name_str, #variant_names, #jvm_param_sig_str)
                            .map_err(instant_coffee::jni_util::map_jni_error)?
                            .l().map_err(instant_coffee::jni_util::map_jni_error)   // This should never error; All Enum variants are objects
                    })*
                }
            }
        };

        from_jni_impl = quote! {
            fn from_jni<'local>(jni_value: jni::objects::JObject<'local>, env: &mut jni::JNIEnv<'local>) -> Result<Self, Option<jni::errors::Exception>> {
                let ordinal = env.call_method(jni_value, "ordinal", "()I", &[])
                    .map_err(instant_coffee::jni_util::map_jni_error)?
                    .i().map_err(instant_coffee::jni_util::map_jni_error)?;   // This shouldn't error; ordinal must return an int

                match ordinal {
                    #(#ordinals => Ok(#name_ident::#variant_idents),)*
                    _ => Err(Some(jni::errors::Exception { class: "java/lang/RuntimeException".to_string(), msg: format!("enum ordinal out of range: {}", ordinal)}))
                }
            }
        };
    };

    let exp = quote! {
        impl #impl_generics instant_coffee::codegen::JavaClass for #name_ident #type_generics #where_clause {
            #class_decl_impl
        }

        impl #impl_generics instant_coffee::JavaType for #name_ident #type_generics #where_clause {
            type JniType<'local> = jni::objects::JObject<'local>;
            type ArrayType<'local> = jni::objects::JObjectArray<'local>;

            fn QUALIFIED_NAME() -> &'static str {#qualified_name_str }

            fn JVM_PARAM_SIGNATURE() -> &'static str { #jvm_param_sig_str }

            fn EXCEPTION_NULL<'local>() -> Self::JniType<'local> { jni::objects::JObject::null() }

            fn from_jvalue<'local>(jvalue: jni::objects::JValueOwned<'local>, _env: &mut jni::JNIEnv<'local>) -> Result<Self::JniType<'local>, Option<jni::errors::Exception>> {
                match jvalue {
                    jni::objects::JValueOwned::Object(obj) => Ok(obj),
                    _ => Err(Some(jni::errors::Exception { class: "java/lang/ClassCastException".to_string(), msg: format!("{} cannot be cast to {}", jvalue.type_name(), <Self as instant_coffee::JavaType>::QUALIFIED_NAME()) }))
                }
            }

            #into_jni_impl

            #from_jni_impl
        }
    };

    Ok(exp.into())
}

#[proc_macro_derive(JavaType)]
pub fn java_type(item: TokenStream) -> TokenStream {
    let class_item = match syn::parse::<ItemStruct>(item.clone()) {
        Ok(item_struct) => Ok(ClassKind::Struct(item_struct)),
        Err(struct_err) => match syn::parse::<ItemEnum>(item) {
            Ok(item_enum) => Ok(ClassKind::Enum(item_enum)),
            Err(enum_err) => {
                // Yield whichever error happens further into the file, as that's the variant we probably should be parsing
                if enum_err.span().unwrap().byte_range().start > struct_err.span().unwrap().byte_range().start {
                    Err(enum_err)
                } else {
                    Err(struct_err)
                }
            }
        }
    };

    class_item.and_then(ClassKind::generate)
        .unwrap_or_else(|err| err.to_compile_error().into())
}

fn is_java_attr(attribute: &Attribute) -> bool {
    match &attribute.meta {
        Meta::List(list) => {
            if list.path.is_ident("derive") {
                list.tokens.to_string() == "JavaType"
            } else {
                false
            }
        }
        _ => false,
    }
}


#[proc_macro_attribute]
pub fn jmodule(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let result: Result<TokenStream, syn::Error> = try {
        let package_literal = syn::parse::<Lit>(attribute)?;
        let package_name = if let Lit::Str(str) = &package_literal {
            let package_name = str.value();
            verify_package_identifier(&package_name).map_err(|e| syn::Error::new(str.span(), e))?;
            package_name
        } else {
            Err(syn::Error::new(package_literal.span(), "Package name must be a string literal"))?
        };

        let mut item_mod = syn::parse::<ItemMod>(item)?;

        if let Some((_, content)) = &mut item_mod.content {
            let mut classes = Vec::new();
            let mut method_map = HashMap::new();

            for item in &mut *content {
                if let Item::Impl(item_impl) = item {
                    if let Type::Path(type_path) = &*item_impl.self_ty {
                        for segment in &type_path.path.segments {
                            match &segment.arguments {
                                PathArguments::None => {}
                                PathArguments::AngleBracketed(arg) => {
                                    Err(syn::Error::new(arg.span(), "generic type impls are not supported"))?;
                                }
                                PathArguments::Parenthesized(arg) => {
                                    Err(syn::Error::new(arg.span(), "function type impls are not supported"))?;
                                }
                            }
                        }
                    } else {
                        Err(syn::Error::new(item_impl.self_ty.span(), "unsupported type for impl block"))?;
                    }

                    let self_type_name = item_impl.self_ty.to_token_stream().to_string();

                    if item_impl.trait_.is_none() {
                        let mut used_types = HashSet::new();
                        let mut used_returns = HashSet::new();
                        let mut exported_functions = Vec::new();
                        for item in &mut item_impl.items {
                            if let ImplItem::Fn(ref mut func) = item {
                                let is_jni_func = func.sig.abi.as_ref()
                                    .and_then(|abi| abi.name.as_ref())
                                    .map(|str| str.value())
                                    .is_some_and(|abi| abi == "jni");

                                if is_jni_func {
                                    if func.sig.asyncness.is_some() {
                                        Err(syn::Error::new(func.sig.asyncness.span(), "async functions are unsupported"))?
                                    }
                                    if func.sig.constness.is_some() {
                                        Err(syn::Error::new(func.sig.constness.span(), "const functions are unsupported"))?
                                    }
                                    if !func.sig.generics.params.is_empty() {
                                        Err(syn::Error::new(func.sig.generics.span(), "generic functions are unsupported"))?
                                    }

                                    func.sig.abi.take();
                                    // if none, this function is static
                                    // if some, this function is a non-static method
                                    let mut self_type: Option<Type> = None;

                                    verify_java_identifier(&func.sig.ident.to_string()).map_err(|e| syn::Error::new(func.sig.ident.span(), e))?;

                                    let mut inputs = Vec::new();
                                    let mut input_mappers = Vec::new();
                                    for input in &func.sig.inputs {
                                        match input {
                                            FnArg::Receiver(receiver) => {
                                                debug_assert!(self_type.is_none(), "duplicate receiver (self) argument?!");
                                                self_type = Some((*receiver.ty).clone());
                                                used_types.insert((*receiver.ty).clone());
                                            }
                                            FnArg::Typed(input_type) => {
                                                let param_name = match &*input_type.pat {
                                                    Pat::Ident(ident) => {
                                                        verify_java_identifier(&ident.ident.to_string()).map_err(|e| syn::Error::new(ident.span(), e))?;

                                                        // Create new ident to redirect Trait-not-implemented errors onto the type's span
                                                        // This de-duplicates trait-not-implemented errors
                                                        Ident::new(&ident.ident.to_string(), input_type.ty.span())
                                                    },
                                                    pattern => {
                                                        Err(syn::Error::new(pattern.span(), "patterns in functions are unsupported"))?
                                                    }
                                                };

                                                used_types.insert((*input_type.ty).clone());
                                                let i_ty = &input_type.ty;
                                                inputs.push(quote!(#param_name: <#i_ty as instant_coffee::JavaType>::JniType<'local>));
                                                input_mappers.push(quote!(<#i_ty as instant_coffee::JavaType>::from_jni(#param_name, &mut env)?));
                                            }
                                        }
                                    }

                                    let output_type = match &func.sig.output {
                                        ReturnType::Default => {
                                            let unit_type_with_span: Type = Type::Tuple(TypeTuple { paren_token: Paren(func.sig.span()), elems: Punctuated::new() });
                                            used_returns.insert(unit_type_with_span.clone());
                                            unit_type_with_span
                                        }
                                        ReturnType::Type(_, return_type) => {
                                            used_returns.insert((**return_type).clone());
                                            (**return_type).clone()
                                        }
                                    };

                                    method_map.entry(item_impl.self_ty.clone())
                                        .or_insert(Vec::new())
                                        .push(func.sig.clone());

                                    let export_name = format!(
                                        "Java_{}_{}_{}",
                                        package_name.replace('_', "_1").replace('.', "_"),
                                        self_type_name.replace('_', "_1"),
                                        func.sig.ident.to_string().replace('_', "_1")
                                    );
                                    let export_ident = Ident::new(&export_name, func.sig.ident.span());

                                    let func_ident = func.sig.ident.clone();

                                    let (self_param, self_mapper) = if let Some(self_type) = self_type {
                                        (
                                            quote!(obj_self: jni::objects::JObject<'local>),
                                            quote!(<#self_type as instant_coffee::JavaType>::from_jni(obj_self, &mut env)?,)
                                        )
                                    } else {
                                        (quote!(class: jni::objects::JClass<'local>), TokenStream::new().into())
                                    };

                                    let export_fn: ImplItemFn = parse_quote! {
                                        #[no_mangle]
                                        pub unsafe extern "system" fn #export_ident<'local>(
                                            mut env: jni::JNIEnv<'local>,
                                            #self_param,
                                            #(#inputs,)*
                                        ) -> <#output_type as instant_coffee::JavaReturn>::JniType<'local> {
                                            let res: Result<<#output_type as instant_coffee::JavaReturn>::JniType<'local>, Option<jni::errors::Exception>> = try {
                                                let out = Self::#func_ident(
                                                    #self_mapper
                                                    #(#input_mappers),*
                                                );

                                                <#output_type as instant_coffee::JavaReturn>::into_jni(out, &mut env)?
                                            };
                                            match res {
                                                Ok(out) => out,
                                                Err(None) => <#output_type as instant_coffee::JavaReturn>::EXCEPTION_NULL(),
                                                Err(Some(exception)) => {
                                                    env.throw_new(exception.class, exception.msg)
                                                        .expect("could not throw exception!");
                                                    <#output_type as instant_coffee::JavaReturn>::EXCEPTION_NULL()
                                                }
                                            }
                                        }
                                    };

                                    exported_functions.push(ImplItem::Fn(export_fn));
                                }
                            }
                        }

                        used_returns.retain(|ret_type| !used_types.contains(ret_type));

                        let new = Vec::with_capacity(item_impl.items.len() + exported_functions.len() + used_types.len() + used_returns.len());
                        let old_items = std::mem::replace(&mut item_impl.items, new);

                        // Bit of a hacky mess, but our type assertions need to be at the top/start of the item list for best errors
                        // RustC generates less helpful errors for the mangled functions
                        std::iter::empty::<ImplItem>()
                            .chain(
                                used_types.into_iter().enumerate().map(|(idx, used_type)| {
                                    let ident = Ident::new(&format!("__ASSERT_TYPE_IMPL_JAVATYPE_{}", idx), proc_macro2::Span::call_site());

                                    parse_quote!(const #ident: fn() -> &'static str = <#used_type as instant_coffee::JavaType>::QUALIFIED_NAME;)
                                })
                            )
                            .chain(
                                used_returns.into_iter().enumerate().map(|(idx, used_return)| {
                                    let ident = Ident::new(&format!("__ASSERT_TYPE_IMPL_JAVARETURN_{}", idx), proc_macro2::Span::call_site());

                                    parse_quote!(const #ident: fn() -> &'static str = <#used_return as instant_coffee::JavaReturn>::QUALIFIED_NAME;)
                                })
                            )
                            .chain(old_items)
                            .chain(exported_functions)
                            .collect_into(&mut item_impl.items);
                    }
                }
            }

            let empty_method_vec = Vec::new();

            // Loop again; We need to have all methods collected first, so cannot do a single pass
            for item in &mut *content {
                match item {
                    Item::Struct(s) if s.attrs.iter().any(is_java_attr) => {
                        let path = Type::Path(TypePath { qself: None, path: Path::from(s.ident.clone()) });
                        let methods = method_map.get(&path).unwrap_or(&empty_method_vec);

                        let package_attr: Attribute = parse_quote!(#[instant_coffee::proc_macro::jmodule_package(#package_name)]);
                        let method_attr: Attribute = parse_quote!(#[instant_coffee::proc_macro::jmodule_methods(#(#methods),*)]);
                        s.attrs.push(package_attr);
                        s.attrs.push(method_attr);
                        classes.push(s.ident.clone());
                    }
                    Item::Enum(e) if e.attrs.iter().any(is_java_attr) => {
                        let path = Type::Path(TypePath { qself: None, path: Path::from(e.ident.clone()) });
                        let methods = method_map.get(&path).unwrap_or(&empty_method_vec);

                        let package_attr: Attribute = parse_quote!(#[instant_coffee::proc_macro::jmodule_package(#package_name)]);
                        let method_attr: Attribute = parse_quote!(#[instant_coffee::proc_macro::jmodule_methods(#(#methods),*)]);
                        e.attrs.push(package_attr);
                        e.attrs.push(method_attr);
                        classes.push(e.ident.clone());
                    }
                    _ => {}
                }
            }

            let module_decl: ItemFn = parse_quote! {
                pub fn jmodule_decl() -> instant_coffee::codegen::JModuleDecl {
                    instant_coffee::codegen::JModuleDecl {
                        name: #package_name,
                        classes: vec![
                            #(<#classes as instant_coffee::codegen::JavaClass>::declaration()),*
                        ]
                    }
                }
            };
            content.push(Item::Fn(module_decl));

            #[cfg(feature = "codegen-ffi")]
            {
                let module_decl_ident = Ident::new(&format!("jmodule_export_{}", package_name.replace('.', "_")), package_literal.span());
                let module_decl_ffi: ItemFn = parse_quote! {
                    #[no_mangle]
                    pub extern "system" fn #module_decl_ident() -> instant_coffee::codegen::FFIJarBlob {
                        jmodule_decl().as_ffi_blob()
                    }
                };
                content.push(Item::Fn(module_decl_ffi));
            }
        } else {
            Err(syn::Error::new(item_mod.span(), "jmodule attribute must be used on an inline module"))?;
        }

        item_mod.into_token_stream().into()
    };

    result.unwrap_or_else(|err| err.into_compile_error().into())
}

/// Attribute to transfer java package information from module-macro to derive macro
#[proc_macro_attribute]
pub fn jmodule_package(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Attribute to transfer assocated-function information from module-macro to derive macro
#[proc_macro_attribute]
pub fn jmodule_methods(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}