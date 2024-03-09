use proc_macro::TokenStream;
use darling::Result;
use quote::quote;
use syn::Fields;

/// # macro for easy handling tera templating
/// need attribute location to get absolute location
/// for example `#[location = "pages/dashboard"]`
/// so on each variant like `Self::Intro`, it will be path
/// `$template_path/pages/dashboard/Intro.html`
#[proc_macro_derive(PageRender,attributes(location))]
pub fn page_render(input: TokenStream) -> TokenStream {
    let enum_ = syn::parse_macro_input!(input as syn::DeriveInput);
    match process_input(enum_) {
        Ok(x) => x,
        Err(err) => err.write_errors().into()
    }
}



#[derive(Debug, Default, darling::FromMeta)]
#[darling(allow_unknown_fields, default)]
struct RenderAttr {
    location: String
}


fn process_input(inp: syn::DeriveInput) -> Result<TokenStream> {
    if let syn::Data::Enum(ref data) = inp.data {
        // extracting the path macro attibutes
        let path = data.variants.iter().map(|variant|{
            let ident = &variant.ident;
            let variant_attr = variant.clone()
                .attrs
                .into_iter()
                .map(|x| darling::ast::NestedMeta::Meta(x.meta))
                .collect::<Vec<_>>();
            let variant_attr = <RenderAttr as darling::FromMeta>::from_list(&variant_attr)
                .expect("is it here?").location;
            match &variant.fields {
                Fields::Unit => {
                    quote! {
                        Self::#ident => #variant_attr.into()
                    }
                }
                Fields::Named(_) => {
                    quote!{
                        Self::#ident {..} => #variant_attr.into()
                    }
                }
                _ => panic!("tuple methode is not supported, its need name values like struct")
            }
        });
        let context = data.variants.iter().map(|variant|{
            let ident = &variant.ident;
            match &variant.fields {
                Fields::Unit => {
                    quote! {
                        Self::#ident => tera::Context::default()
                    }
                }
                Fields::Named(fields) => {
                    let key = fields.named.iter().map(|f|f.ident.as_ref());
                    let key2 = key.clone();
                    let key3 = key.clone();
                    quote! {
                        Self::#ident { #(#key),* } => {
                            let mut ctx = tera::Context::default();
                            #(ctx.insert(stringify!(#key2),#key3);)*
                            ctx
                        }
                    }
                }
                _ => panic!("tuple methode are not allowed")
            }
        });

        let ident = &inp.ident;

        Ok(quote! {
            impl crate::PageRender for #ident {
                fn path(&self) -> String {
                    match self {
                        #(#path,)*
                    }
                }
                fn context(&self) -> tera::Context {
                    match self {
                        #(#context,)*
                    }
                }
            }
        }.into())

    }else {
        Err(darling::Error::custom("can only usable on enum"))
    }
}
