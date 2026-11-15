//! The attribute of the app's commands (docs/REMOTE.md, « La fondation commune », point 2): a `#[command]` stays the
//! Tauri command it was, and also gives its entry of the remote table (`remoteapi::table` of the app), which calls it
//! with JSON without Tauri. `handlers!` gives both lists from one, for the two never to differ. Both only work inside
//! the app's crate: what they write names `crate::remoteapi::table`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as Tokens;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Error, FnArg, Ident, ItemFn, Pat, Path, Token, Type};

/// `#[command]`, `#[command(async)]`, `#[command(desktop)]`, `#[command(async, desktop)]`: the function under
/// `#[tauri::command]` (`async` as Tauri's), and `__REMOTE_<NAME>`, its entry of the remote table. Remotely, the core
/// is given for a `CoreState` parameter and every other one is read from the JSON object under Tauri's key; a
/// parameter only the window has (`AppHandle`, `State`, `Channel`…) is refused unless the command is `desktop`, whose
/// entry says it is not called remotely.
#[proc_macro_attribute]
pub fn command(args: TokenStream, item: TokenStream) -> TokenStream {
    expand_command(args.into(), item.into())
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// `handlers![commands::a, commands::b]`: `(tauri::generate_handler![commands::a, commands::b], &[their entries])`.
/// Bare paths only: no attribute on an item (a `#[cfg]`), no `#![plugin(…)]` as `generate_handler!` takes.
/// Every item must be a `#[command]`: a `#[tauri::command]` in the list fails with « cannot find value
/// `__REMOTE_…` ».
#[proc_macro]
pub fn handlers(input: TokenStream) -> TokenStream {
    expand_handlers(input.into())
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_command(args: Tokens, item: Tokens) -> syn::Result<Tokens> {
    let words = (|input: ParseStream| {
        Punctuated::<Ident, Token![,]>::parse_terminated_with(input, Ident::parse_any)
    })
    .parse2(args)?;
    let (mut run_async, mut desktop) = (false, false);
    for word in words {
        match word.to_string().as_str() {
            "async" => run_async = true,
            "desktop" => desktop = true,
            _ => return Err(Error::new(word.span(), "expected `async` or `desktop`")),
        }
    }
    let function: ItemFn = syn::parse2(item)?;
    let tauri = if run_async {
        quote!(#[::tauri::command(async)])
    } else {
        quote!(#[::tauri::command])
    };
    let entry = entry(&function, desktop).unwrap_or_else(Error::into_compile_error);
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = function;
    // Tauri's attribute after the command's own, where it was: a `cfg` among them still goes first.
    Ok(quote! {
        #(#attrs)*
        #tauri
        #vis #sig #block
        #entry
    })
}

/// `__REMOTE_<NAME>`, under the command's `cfg` and with its visibility.
fn entry(function: &ItemFn, desktop: bool) -> syn::Result<Tokens> {
    let name = function.sig.ident.to_string();
    let constant = remote_const(&function.sig.ident);
    let vis = &function.vis;
    let cfgs = function.attrs.iter().filter(|a| a.path().is_ident("cfg"));
    let call = if desktop {
        quote!(::std::option::Option::None)
    } else {
        let call = remote_call(function)?;
        quote!(::std::option::Option::Some(#call))
    };
    Ok(quote! {
        #(#cfgs)*
        #[doc(hidden)]
        #vis const #constant: crate::remoteapi::table::Entry = crate::remoteapi::table::Entry {
            name: #name,
            call: #call,
        };
    })
}

/// The function of the entry: the arguments read, the command awaited (`async fn`) or run on a thread for what blocks
/// (`fn`), its value answered as Tauri answers it.
fn remote_call(function: &ItemFn) -> syn::Result<Tokens> {
    let sig = &function.sig;
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        return Err(Error::new(
            sig.generics.span(),
            "a generic command cannot be called remotely: mark it `#[command(desktop)]`",
        ));
    }
    let command = &sig.ident;
    let name = command.to_string();
    let mut reads = Vec::new();
    let mut passed = Vec::new();
    for (i, input) in sig.inputs.iter().enumerate() {
        let FnArg::Typed(arg) = input else {
            return Err(Error::new(input.span(), "a command cannot take `self`"));
        };
        match classify(&arg.ty) {
            Param::Core => passed.push(quote!(::std::convert::From::from(&ctx.core))),
            Param::Desktop(ty) => {
                return Err(Error::new(
                    arg.ty.span(),
                    format!(
                        "`{ty}` exists only in the desktop app: mark the command `#[command(desktop)]`, \
                         or remove this parameter"
                    ),
                ))
            }
            Param::Arg => {
                let key = key(&arg.pat)?;
                let var = format_ident!("__arg{i}");
                let ty = &arg.ty;
                reads.push(quote! {
                    let #var: #ty = crate::remoteapi::table::arg(&args, #name, #key)?;
                });
                passed.push(quote!(#var));
            }
        }
    }
    let run = if sig.asyncness.is_some() {
        quote! {
            let r = #command(#(#passed),*).await;
            (&r).reply_kind().reply(r)
        }
    } else {
        quote! {
            crate::remoteapi::table::blocking(move || {
                let r = #command(#(#passed),*);
                (&r).reply_kind().reply(r)
            })
            .await?
        }
    };
    Ok(quote! {{
        #[allow(unused_variables)]
        fn __remote_call(
            ctx: crate::remoteapi::table::Ctx,
            args: ::serde_json::Value,
        ) -> crate::remoteapi::table::Pending {
            ::std::boxed::Box::pin(async move {
                #[allow(unused_imports)]
                use crate::remoteapi::table::{ReplyResult as _, ReplyValue as _};
                #(#reads)*
                #run
            })
        }
        __remote_call as crate::remoteapi::table::Call
    }})
}

/// What a parameter is remotely, by its type's name: a proc macro cannot resolve types.
#[derive(Debug, PartialEq)]
enum Param {
    /// `CoreState`: the core of the connection.
    Core,
    /// Only the window has it: its name.
    Desktop(String),
    /// Read from the JSON object.
    Arg,
}

/// The types of the parameters only the window has.
const DESKTOP: &[&str] = &[
    "AppHandle",
    "State",
    "VoiceState",
    "Channel",
    "Window",
    "WebviewWindow",
    "Webview",
    "Request",
];

fn classify(ty: &Type) -> Param {
    match ty {
        Type::Reference(r) => classify(&r.elem),
        Type::Path(p) => match p.path.segments.last() {
            Some(last) if last.ident == "CoreState" => Param::Core,
            Some(last) if DESKTOP.iter().any(|d| last.ident == d) => {
                Param::Desktop(last.ident.to_string())
            }
            _ => Param::Arg,
        },
        _ => Param::Arg,
    }
}

/// The key Tauri reads a parameter from.
fn key(pat: &Pat) -> syn::Result<String> {
    let Pat::Ident(p) = pat else {
        return Err(Error::new(
            pat.span(),
            "only a named parameter can be read remotely: name it, or mark the command `#[command(desktop)]`",
        ));
    };
    camel(&p.ident.unraw().to_string()).ok_or_else(|| {
        Error::new(
            p.ident.span(),
            "a parameter read remotely is named in snake_case (lowercase ASCII, digits, `_`): rename it, or mark \
             the command `#[command(desktop)]`",
        )
    })
}

/// `#[tauri::command]`'s key: heck's lower camel case (tauri-macros 2.7.0, `command/wrapper.rs`, `parse_arg`), for
/// the names whose result is certain here: lowercase ASCII, digits and `_`. Words split at `_`, empty ones dropped,
/// the first as it is and the others with their first letter in uppercase.
fn camel(name: &str) -> Option<String> {
    if !name
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return None;
    }
    let mut words = name.split('_').filter(|w| !w.is_empty());
    let mut key = words.next()?.to_string();
    for word in words {
        key.push_str(&word[..1].to_ascii_uppercase());
        key.push_str(&word[1..]);
    }
    Some(key)
}

/// The entry's constant of a command: `quit_app` → `__REMOTE_QUIT_APP`.
fn remote_const(command: &Ident) -> Ident {
    format_ident!(
        "__REMOTE_{}",
        command.unraw().to_string().to_ascii_uppercase()
    )
}

fn expand_handlers(input: Tokens) -> syn::Result<Tokens> {
    let paths = Punctuated::<Path, Token![,]>::parse_terminated.parse2(input)?;
    let entries = paths.iter().map(|path| {
        let mut path = path.clone();
        if let Some(last) = path.segments.last_mut() {
            last.ident = remote_const(&last.ident);
        }
        path
    });
    let commands = paths.iter();
    Ok(quote! {
        (
            ::tauri::generate_handler![#(#commands),*],
            {
                const ENTRIES: &[crate::remoteapi::table::Entry] = &[#(#entries),*];
                ENTRIES
            },
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ty(text: &str) -> Type {
        syn::parse_str(text).unwrap()
    }

    #[test]
    fn a_parameter_is_classified_by_its_type_name() {
        let desktop = |name: &str| Param::Desktop(name.into());
        let cases = [
            ("CoreState", Param::Core),
            ("CoreState<'_>", Param::Core),
            ("crate::commands::CoreState<'a>", Param::Core),
            ("tauri::AppHandle", desktop("AppHandle")),
            ("AppHandle<R>", desktop("AppHandle")),
            ("&tauri::AppHandle", desktop("AppHandle")),
            ("State<crate::updates::Updates>", desktop("State")),
            ("tauri::State<'_, Arc<Core>>", desktop("State")),
            ("VoiceState<'_>", desktop("VoiceState")),
            ("Channel<InvokeResponseBody>", desktop("Channel")),
            ("tauri::ipc::Channel<UiEvent>", desktop("Channel")),
            ("tauri::Window", desktop("Window")),
            ("tauri::WebviewWindow", desktop("WebviewWindow")),
            ("tauri::Webview", desktop("Webview")),
            ("tauri::ipc::Request", desktop("Request")),
            ("String", Param::Arg),
            ("Option<String>", Param::Arg),
            ("Option<CoreState>", Param::Arg),
            ("Vec<EngineId>", Param::Arg),
            ("u32", Param::Arg),
            ("ReviewState", Param::Arg),
            ("serde_json::Value", Param::Arg),
            ("(u16, u16)", Param::Arg),
        ];
        for (text, want) in cases {
            assert_eq!(classify(&ty(text)), want, "{text}");
        }
    }

    #[test]
    fn a_key_is_the_one_tauri_reads() {
        let cases = [
            ("id", Some("id")),
            ("n", Some("n")),
            ("agent_id", Some("agentId")),
            ("media_type", Some("mediaType")),
            ("request_id", Some("requestId")),
            ("agent_2_id", Some("agent2Id")),
            ("v2_name", Some("v2Name")),
            ("x_2d", Some("x2d")),
            ("_core", Some("core")),
            ("a__b_", Some("aB")),
            ("__", None),
            ("agentId", None),
            ("été", None),
        ];
        for (name, want) in cases {
            assert_eq!(camel(name).as_deref(), want, "{name}");
        }
        let raw: Pat = Pat::parse_single.parse_str("r#type").unwrap();
        assert_eq!(key(&raw).unwrap(), "type");
        let wild: Pat = Pat::parse_single.parse_str("_").unwrap();
        assert!(key(&wild).is_err());
    }

    #[test]
    fn an_entry_is_named_from_its_command() {
        let id = |s: &str| syn::parse_str::<Ident>(s).unwrap();
        assert_eq!(remote_const(&id("quit_app")), "__REMOTE_QUIT_APP");
        assert_eq!(remote_const(&id("r#type")), "__REMOTE_TYPE");
    }

    #[test]
    fn handlers_give_tauris_list_and_the_entries() {
        let got = expand_handlers(quote!(commands::set_ui, commands::quit_app,)).unwrap();
        let want = quote! {
            (
                ::tauri::generate_handler![commands::set_ui, commands::quit_app],
                {
                    const ENTRIES: &[crate::remoteapi::table::Entry] =
                        &[commands::__REMOTE_SET_UI, commands::__REMOTE_QUIT_APP];
                    ENTRIES
                },
            )
        };
        assert_eq!(got.to_string(), want.to_string());
    }

    fn expand(args: Tokens, item: Tokens) -> String {
        expand_command(args, item)
            .unwrap_or_else(Error::into_compile_error)
            .to_string()
    }

    #[test]
    fn a_desktop_parameter_is_refused_unless_the_command_is_desktop() {
        let item = quote!(
            pub fn quit_app(app: tauri::AppHandle) {}
        );
        let refused = expand(quote!(), item.clone());
        assert!(refused.contains("compile_error"), "{refused}");
        assert!(
            refused.contains("`AppHandle` exists only in the desktop app"),
            "{refused}"
        );
        // The command stays, for its Tauri side to compile.
        assert!(refused.contains("fn quit_app"), "{refused}");

        let desktop = expand(quote!(desktop), item);
        assert!(!desktop.contains("compile_error"), "{desktop}");
        assert!(
            desktop.contains("call : :: std :: option :: Option :: None"),
            "{desktop}"
        );
        // Any command may be desktop, even one the table could call.
        let plain = expand(
            quote!(desktop),
            quote!(
                fn n(n: u32) {}
            ),
        );
        assert!(plain.contains("Option :: None"), "{plain}");
    }

    #[test]
    fn a_generic_command_is_refused_unless_desktop() {
        let item = quote!(
            fn f<R: tauri::Runtime>(app: tauri::AppHandle<R>) {}
        );
        assert!(
            expand(quote!(), item.clone()).contains("a generic command cannot be called remotely")
        );
        assert!(!expand(quote!(desktop), item).contains("compile_error"));
    }

    #[test]
    fn the_command_keeps_its_attributes_and_the_entry_its_cfg_and_visibility() {
        let got = expand(
            quote!(async),
            quote!(
                /// Doc.
                #[cfg(target_os = "macos")]
                #[allow(clippy::too_many_arguments)]
                pub(crate) fn f(core: CoreState, agent_id: String) {}
            ),
        );
        let want_command = quote! {
            #[doc = r" Doc."]
            #[cfg(target_os = "macos")]
            #[allow(clippy::too_many_arguments)]
            #[::tauri::command(async)]
            pub(crate) fn f(core: CoreState, agent_id: String) {}
            #[cfg(target_os = "macos")]
            #[doc(hidden)]
            pub(crate) const __REMOTE_F: crate::remoteapi::table::Entry
        };
        assert!(got.starts_with(&want_command.to_string()), "{got}");
        assert!(got.contains("arg (& args , \"f\" , \"agentId\")"), "{got}");
        assert!(
            got.contains("crate :: remoteapi :: table :: blocking"),
            "{got}"
        );
        let awaited = expand(
            quote!(),
            quote!(
                async fn g(core: CoreState<'_>) {}
            ),
        );
        assert!(
            awaited.starts_with("# [:: tauri :: command] async fn g"),
            "{awaited}"
        );
        assert!(
            awaited.contains("g (:: std :: convert :: From :: from (& ctx . core)) . await"),
            "{awaited}"
        );
    }

    #[test]
    fn unknown_options_and_self_are_refused() {
        assert!(expand(
            quote!(rename_all = "snake_case"),
            quote!(
                fn f() {}
            )
        )
        .contains("compile_error"));
        assert!(expand(
            quote!(remote),
            quote!(
                fn f() {}
            )
        )
        .contains("expected `async` or `desktop`"));
        assert!(expand(
            quote!(),
            quote!(
                fn f(&self) {}
            )
        )
        .contains("a command cannot take `self`"));
    }
}
