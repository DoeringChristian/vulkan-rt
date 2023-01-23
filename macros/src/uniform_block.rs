use quote::quote;

pub fn generate_uniform_block(ast: syn::DeriveInput) -> proc_macro2::TokenStream {
    let ident = ast.ident;

    if let syn::Data::Struct(syn::DataStruct { fields, .. }) = ast.data {
        let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

        quote! {
            impl #impl_generics UniformBlock for #ident #ty_generics #where_clause{
                fn write_to(&self, dst: &mut impl std::io::Write){

                }
            }
        }
    } else {
        panic!("Onyl structs are supported for Uniform Blocks");
    }
}
