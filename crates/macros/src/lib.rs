//! Procedural macros for pray.rs.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    Error, FnArg, Ident, ItemFn, Pat, Token,
};

/// Require a signed-in user in a server function and bind what it needs.
///
/// ```ignore
/// #[authed]                      // ctx: Authed  (user_id, pool(), session, state)
/// #[authed(user, pool)]          // user: UserId, pool: SqlitePool
/// #[authed(user as author)]      // rename a binding (rarely needed)
/// #[server]
/// pub async fn fetch_posts(filter: VisibilityFilter) -> Result<Vec<ViewedPost>, ServerFnError> {
///     posts::list_for_viewer(&pool, user, &filter).await …
/// }
/// ```
///
/// Bindings:
///
/// | name      | type                        |
/// |-----------|-----------------------------|
/// | `ctx`     | `crate::server::Authed`     |
/// | `user`    | `UserId`                    |
/// | `pool`    | `sqlx::SqlitePool`          |
/// | `session` | `tower_sessions::Session`   |
/// | `state`   | `crate::server::AppState`   |
///
/// Expands to one statement at the top of the body that extracts
/// [`Authed`](crate::server::Authed) — failing with "not authenticated" when
/// nobody is signed in — followed by the requested bindings.
///
/// Must sit **above** `#[server]`: it rewrites the body before `#[server]`
/// splits the function into its server and client halves, so the generated
/// code only exists on the server.
#[proc_macro_attribute]
pub fn authed(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let func = parse_macro_input!(item as ItemFn);
    expand(args, func).unwrap_or_else(Error::into_compile_error).into()
}

/// What a binding exposes from the request context.
#[derive(Clone, Copy, PartialEq)]
enum Field {
    Ctx,
    User,
    Pool,
    Session,
    State,
}

impl Field {
    const ALL: [(&'static str, Field); 5] = [
        ("ctx", Field::Ctx),
        ("user", Field::User),
        ("pool", Field::Pool),
        ("session", Field::Session),
        ("state", Field::State),
    ];

    fn parse(ident: &Ident) -> syn::Result<Field> {
        let name = ident.to_string();
        Field::ALL
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, f)| *f)
            .ok_or_else(|| {
                let valid: Vec<_> = Field::ALL.iter().map(|(n, _)| format!("`{n}`")).collect();
                Error::new(
                    ident.span(),
                    format!("unknown binding `{name}`; expected one of {}", valid.join(", ")),
                )
            })
    }
}

/// `field` or `field as name`.
struct Binding {
    field: Field,
    field_span: Span,
    name: Ident,
}

impl Parse for Binding {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        let field = Field::parse(&ident)?;
        let name = if input.peek(Token![as]) {
            input.parse::<Token![as]>()?;
            input.parse()?
        } else {
            ident.clone()
        };
        Ok(Binding { field, field_span: ident.span(), name })
    }
}

struct Args(Vec<Binding>);

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let list = Punctuated::<Binding, Token![,]>::parse_terminated(input)?;
        Ok(Args(list.into_iter().collect()))
    }
}

fn expand(args: Args, mut func: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let sig = &func.sig;
    if sig.asyncness.is_none() {
        return Err(Error::new(sig.fn_token.span(), "#[authed] needs an `async fn`"));
    }
    if !func.attrs.iter().any(|a| a.path().segments.last().is_some_and(|s| s.ident == "server")) {
        return Err(Error::new(
            sig.ident.span(),
            "#[authed] must sit directly above #[server]: it rewrites the body before \
             #[server] splits the function into server and client halves",
        ));
    }

    // No arguments: bind the whole context as `ctx`.
    let bindings = if args.0.is_empty() {
        vec![Binding { field: Field::Ctx, field_span: Span::call_site(), name: Ident::new("ctx", Span::call_site()) }]
    } else {
        args.0
    };

    // Names must be unique and must not shadow the function's parameters.
    let params: Vec<Ident> = sig.inputs.iter().filter_map(|arg| match arg {
        FnArg::Typed(t) => match &*t.pat {
            Pat::Ident(p) => Some(p.ident.clone()),
            _ => None,
        },
        FnArg::Receiver(_) => None,
    }).collect();
    let mut errors: Option<Error> = None;
    let mut push = |e: Error| match &mut errors {
        Some(all) => all.combine(e),
        None => errors = Some(e),
    };
    for (i, b) in bindings.iter().enumerate() {
        if bindings[..i].iter().any(|prev| prev.name == b.name) {
            push(Error::new(b.name.span(), format!("`{}` is bound twice", b.name)));
        }
        // Same field under a new name (e.g. `user, user as u`); a plain
        // repeat is already reported as "bound twice".
        if let Some(prev) = bindings[..i].iter().find(|prev| prev.field == b.field && prev.name != b.name) {
            push(Error::new(b.field_span, format!("already bound as `{}`; bind each field once", prev.name)));
        }
        if params.contains(&b.name) {
            push(Error::new(
                b.name.span(),
                format!("`{}` would shadow the parameter of the same name; rename it with `as`", b.name),
            ));
        }
    }
    if let Some(e) = errors {
        return Err(e);
    }

    // Owned bindings (cheap clones: pool/session/state are Arc-backed), with
    // `ctx` moved last so nothing borrows from it.
    let authed = Ident::new("__authed", Span::mixed_site());
    let mut lets = Vec::new();
    let mut ctx_let = None;
    for Binding { field, name, .. } in &bindings {
        match field {
            Field::User    => lets.push(quote! { let #name = #authed.user_id; }),
            Field::Pool    => lets.push(quote! { let #name = #authed.pool().clone(); }),
            Field::Session => lets.push(quote! { let #name = #authed.session.clone(); }),
            Field::State   => lets.push(quote! { let #name = #authed.state.clone(); }),
            Field::Ctx     => ctx_let = Some(quote! { let #name = #authed; }),
        }
    }

    let body = &func.block;
    func.block = syn::parse_quote!({
        let #authed = crate::server::extract_authed().await?;
        #(#lets)*
        #ctx_let
        #body
    });
    Ok(quote!(#func))
}
