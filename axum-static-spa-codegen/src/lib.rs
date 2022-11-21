use proc_macro2::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use quote::quote;
use std::fs;
use walkdir::WalkDir;

fn content_to_stream(content: &[u8]) -> TokenStream {
    let byte_punct = TokenStream::from_iter(content.iter().flat_map::<[TokenTree; 2], _>(|byte| {
        [
            Literal::u8_suffixed(*byte).into(),
            Punct::new(',', Spacing::Alone).into(),
        ]
    }));

    TokenTree::from(Group::new(Delimiter::Bracket, byte_punct)).into()
}

/// Generates an axum router for SPAs (single-page applications)
///
/// Example:
///
/// ```ignore
/// let spa_router = axum_static_spa::include_dist!("frontend/dist");
/// ```
#[proc_macro]
pub fn include_dist(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: TokenStream = input.into();
    let TokenTree::Literal(dist_dir) = input.into_iter().next().expect("Missing path to dist directory") else {
        panic!("Expected literal path");
    };
    let dist_dir = dist_dir.to_string().trim_matches('"').to_owned();
    let walk_dir = WalkDir::new(&dist_dir);

    let mut names = Vec::new();
    let mut contents = Vec::new();
    for entry in walk_dir.into_iter() {
        let entry = entry.expect("Failed to walk dist directory");
        if !entry.path().is_file() {
            continue;
        }

        let file_name_str = entry
            .path()
            .to_str()
            .expect("Non UTF-8 filename")
            .strip_prefix(&dist_dir)
            .unwrap()
            .to_string();
        let content = fs::read(entry.path()).expect("Failed to read file");

        names.push(file_name_str);
        contents.push(content_to_stream(&content));
    }

    let idx = names
        .iter()
        .position(|name| name == "/index.html")
        .expect("Missing index.html");
    let index_contents = &contents[idx];

    quote! {
        {
            #[allow(clippy::all)]
            {
                ::axum_static_spa::Router::new()
                    #(
                        .route(#names, ::axum_static_spa::get(|| async { const CONTENT: &[u8] = &#contents; CONTENT }))
                    )*
                    .fallback(::axum_static_spa::Handler::into_service(|| async { const CONTENT: &[u8] = &#index_contents; ::axum_static_spa::Html(CONTENT) }))
            }
        }
    }
    .into()
}
