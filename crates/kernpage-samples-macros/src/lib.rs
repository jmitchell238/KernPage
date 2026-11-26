//! `#[kernpage_samples::test]`: a `#[test]` that passes (after printing a skip line) when a sample
//! file it asked for through `kernpage_samples` is not available on this machine.

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

#[proc_macro_attribute]
pub fn test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut tokens: Vec<TokenTree> = item.into_iter().collect();
    // The function body is the last brace group.
    let Some(TokenTree::Group(body)) = tokens.pop() else {
        panic!("#[kernpage_samples::test] must be put on a function");
    };
    assert!(
        body.delimiter() == Delimiter::Brace,
        "#[kernpage_samples::test] must be put on a function with a body"
    );
    // `{ ::kernpage_samples::run(|| <body>) }`
    let mut call: TokenStream = "::kernpage_samples::run".parse().unwrap();
    let mut arg: TokenStream = "||".parse().unwrap();
    arg.extend(std::iter::once(TokenTree::Group(body)));
    call.extend(std::iter::once(TokenTree::Group(Group::new(
        Delimiter::Parenthesis,
        arg,
    ))));
    let wrapped = call;
    let mut out: TokenStream = "#[test]".parse().unwrap();
    out.extend(tokens);
    out.extend(std::iter::once(TokenTree::Group(Group::new(
        Delimiter::Brace,
        wrapped,
    ))));
    out
}
