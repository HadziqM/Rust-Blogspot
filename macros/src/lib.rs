use darling::Result;
use proc_macro::TokenStream;
use quote::quote;
use syn::Fields;

/// # macro for easy handling tera templating
/// need attribute location to get absolute location
/// for example `#[location = "pages/dashboard"]`
/// so on each variant like `Self::Intro`, it will be path
/// `$template_path/pages/dashboard/Intro.html`
/// ```rust
/// #[derive(PageRender)]
/// pub enum Pages {
///     #[location = "pages/404.html"]
///     #[error_page]
///     E404,
///     #[location = "pages/blog.html"]
///     Post { post: String },
/// }
/// ````
#[proc_macro_derive(PageRender, attributes(location, error_page))]
pub fn page_render(input: TokenStream) -> TokenStream {
    let enum_ = syn::parse_macro_input!(input as syn::DeriveInput);
    match process_input(enum_) {
        Ok(x) => x,
        Err(err) => err.write_errors().into(),
    }
}

#[derive(Debug, Default, darling::FromMeta)]
#[darling(allow_unknown_fields, default)]
struct RenderAttr {
    location: String,
    error_page: Option<()>,
}

fn process_input(inp: syn::DeriveInput) -> Result<TokenStream> {
    if let syn::Data::Enum(ref data) = inp.data {
        // extracting the path macro attibutes
        let mut err_page = None;
        for variant in &data.variants {
            let ident = &variant.ident;
            let variant_attr = variant
                .clone()
                .attrs
                .into_iter()
                .map(|x| darling::ast::NestedMeta::Meta(x.meta))
                .collect::<Vec<_>>();
            let variant_attr =
                <RenderAttr as darling::FromMeta>::from_list(&variant_attr).expect("is it here?");
            match &variant.fields {
                Fields::Unit => {
                    if variant_attr.error_page.is_some() {
                        err_page = Some(ident.clone());
                    }
                }
                Fields::Named(_) => {
                    if variant_attr.error_page.is_some() {
                        panic!("cant use error page with struct, this is for simple error page without any context, make custom one instead");
                    }
                }
                _ => panic!("tuple methode is not supported, its need name values like struct"),
            };
        }
        let path = data.variants.iter().map(|variant| {
            let ident = &variant.ident;
            let variant_attr = variant
                .clone()
                .attrs
                .into_iter()
                .map(|x| darling::ast::NestedMeta::Meta(x.meta))
                .collect::<Vec<_>>();
            let variant_attr =
                <RenderAttr as darling::FromMeta>::from_list(&variant_attr).expect("is it here?");
            let location = &variant_attr.location;
            match &variant.fields {
                Fields::Unit => {
                    quote! {
                        Self::#ident => #location.into()
                    }
                }
                Fields::Named(_) => {
                    quote! {
                        Self::#ident {..} => #location.into()
                    }
                }
                _ => panic!("tuple methode is not supported, its need name values like struct"),
            }
        });
        let context = data.variants.iter().map(|variant| {
            let ident = &variant.ident;
            match &variant.fields {
                Fields::Unit => {
                    quote! {
                        Self::#ident => template::tera::Context::default()
                    }
                }
                Fields::Named(fields) => {
                    let key = fields.named.iter().map(|f| f.ident.as_ref());
                    let key2 = key.clone();
                    let key3 = key.clone();
                    quote! {
                        Self::#ident { #(#key),* } => {
                            let mut ctx = template::tera::Context::default();
                            #(ctx.insert(stringify!(#key2),#key3);)*
                            ctx
                        }
                    }
                }
                _ => panic!("tuple methode are not allowed"),
            }
        });

        let ident = &inp.ident;
        let quoted_err = match &err_page {
            Some(x) => quote! {Some(Self::#x)},
            None => quote! {None},
        };

        Ok(quote! {
            impl template::PageRender for #ident {
                fn path(&self) -> String {
                    match self {
                        #(#path,)*
                    }
                }
                fn context(&self) -> template::tera::Context {
                    match self {
                        #(#context,)*
                    }
                }
                fn err_page(&self) -> Option<Self> {
                    #quoted_err
                }
            }
        }
        .into())
    } else {
        Err(darling::Error::custom("can only usable on enum"))
    }
}
