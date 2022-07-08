mod uniform_block;

#[proc_macro_derive(UniformBlock)]
pub fn derive_uniform_block(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast: syn::DeriveInput = syn::parse(tokens).unwrap();

    uniform_block::generate_uniform_block(ast).into()
}
