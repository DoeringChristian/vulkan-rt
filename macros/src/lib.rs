use quote::quote;
use syn::parse_macro_input;
use syn::Attribute;

#[proc_macro_derive(ReprGlsl)]
pub fn derive_repr_glsl(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(tokens).unwrap();

    let ident = ast.ident;
    if let syn::Data::Struct(syn::DataStruct { fields, .. }) = ast.data {
        let sources = fields
            .iter()
            .map(|field| match field.ty.clone() {
                syn::Type::Path(path) => path,
                _ => unimplemented!(),
            })
            .collect::<Vec<_>>();

        let mut format_str = fields
            .iter()
            .map(|field| {
                format!(
                    "\t{{{{}}}} {};\n",
                    field.ident.as_ref().unwrap().to_string()
                )
            })
            .fold(format!("struct {}", ident.to_string()), |mut a, b| {
                a.push_str(&b);
                a
            });
        format_str.push_str("}};\n");

        let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
        quote! {
            impl #impl_generics ReprGlsl for #ident #ty_generics #where_clause{
                const fn glsl_source() -> &'static str{
                    format!(#(#sources ::glsl_source()),*);
                }
            }
        }
        .into()
    } else {
        panic!("Only structs are supported!");
    }
}
