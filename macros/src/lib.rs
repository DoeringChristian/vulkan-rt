use quote::quote;

#[proc_macro_derive(AsGlsl)]
pub fn derive_as_glsl(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(tokens).unwrap();

    let ident = ast.ident;
    if let syn::Data::Struct(syn::DataStruct { fields, .. }) = ast.data {
        let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
        quote! {
            impl #impl_generics AsGlsl for #ident #ty_generics #where_clause{
                const GLSL_SOURCE: str = "";
            }
        }
        .into()
    } else {
        panic!("Only structs are supported!");
    }
}
