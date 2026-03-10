use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Data, DataStruct, Fields, Type};

#[proc_macro_derive(BuildArgv, attributes(arg))]
pub fn build_argv_derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(DataStruct { fields: Fields::Named(ref f), .. }) => &f.named,
        _ => {
            return syn::Error::new_spanned(name, "BuildArgv only supports structs with named fields")
                .to_compile_error()
                .into();
        }
    };

    let mut stmts = Vec::new();

    for f in fields.iter() {
        let ident = f.ident.as_ref().unwrap();
        let fname = ident.to_string();

        // Defaults
        let mut short: Option<char> = None;
        let mut long: Option<String> = None;
        let mut has_value_names = false;

        // parse #[arg(...)] attributes if present by token inspection (simple)
        for attr in &f.attrs {
            if attr.path().is_ident("arg") {
                let tokens = quote::quote!(#attr).to_string();
                // look for short = 'x' or short='x'
                if let Some(pos) = tokens.find("short") {
                    if let Some(eq) = tokens[pos..].find('=') {
                        let rest = &tokens[pos + eq + 1..];
                        if let Some(ch_pos) = rest.find('\'') {
                            // find next char between single quotes
                            let chars: Vec<char> = rest.chars().collect();
                            if ch_pos + 2 < chars.len() {
                                short = Some(chars[ch_pos + 1]);
                            }
                        } else if let Some(dq_pos) = rest.find('"') {
                            // short may be given as string
                            let chars: Vec<char> = rest.chars().collect();
                            if dq_pos + 1 < chars.len() {
                                short = Some(chars[dq_pos + 1]);
                            }
                        }
                    }
                }
                // look for long = "..." or long="..."
                if let Some(pos) = tokens.find("long") {
                    if let Some(eq) = tokens[pos..].find('=') {
                        let rest = &tokens[pos + eq + 1..];
                        if let Some(start) = rest.find('"') {
                            if let Some(end) = rest[start + 1..].find('"') {
                                long = Some(rest[start + 1..start + 1 + end].to_string());
                            }
                        }
                    }
                }
                if tokens.contains("value_names") {
                    has_value_names = true;
                }
            }
        }

        // pick token form: prefer short, then long, else fallback to --{field}
        let token = if let Some(c) = short {
            let s = c.to_string();
            quote! { format!("-{}", #s) }
        } else if let Some(l) = long.clone() {
            quote! { format!("--{}", #l) }
        } else {
            // fallback uses field name as long form
            quote! { format!("--{}", #fname) }
        };

        // Determine type
        let ty = &f.ty;

        // bool
        if let Type::Path(tp) = ty {
            let seg = tp.path.segments.last().unwrap().ident.to_string();
            if seg == "bool" {
                stmts.push(quote! {
                    if self.#ident {
                        argv.push(#token);
                    }
                });
                continue;
            }
            // Option<T>
            if seg == "Option" {
                stmts.push(quote! {
                    if let Some(ref v) = self.#ident {
                        argv.push(#token);
                        argv.push(v.clone());
                    }
                });
                continue;
            }
            // Vec<T>
            if seg == "Vec" {
                // detect pair semantics (value_names or common names)
                let is_pair = has_value_names || ["load", "set_token", "put_data"].contains(&fname.as_str());
                if is_pair {
                    stmts.push(quote! {
                        for chunk in self.#ident.chunks(2) {
                            if chunk.len() == 2 {
                                argv.push(#token);
                                argv.push(chunk[0].clone());
                                argv.push(chunk[1].clone());
                            }
                        }
                    });
                } else {
                    stmts.push(quote! {
                        for v in &self.#ident {
                            argv.push(#token);
                            argv.push(v.clone());
                        }
                    });
                }
                continue;
            }
            // String (or other Path) treat as single
            if seg == "String" {
                stmts.push(quote! {
                    argv.push(#token);
                    argv.push(self.#ident.clone());
                });
                continue;
            }
        }

        // Fallback: try to stringify value
        stmts.push(quote! {
            // fallback: push token and stringified value
            argv.push(#token);
            argv.push(format!("{}", &self.#ident));
        });
    }

    let gen = quote! {
        impl #name {
            pub fn build_argv(&self) -> Vec<String> {
                let mut argv: Vec<String> = Vec::new();
                argv.push("snapshot".to_string());
                #(#stmts)*
                argv
            }
        }
    };

    gen.into()
}

