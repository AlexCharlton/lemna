extern crate proc_macro;

use proc_macro::{Group, TokenStream, TokenTree};
use quote::quote;
use std::iter::FromIterator;
use syn::{self, parse_macro_input};

#[proc_macro_attribute]
pub fn state_component(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as syn::AttributeArgs);
    let state_type = attr.first().unwrap();

    let expanded = quote! {
        state: Option<#state_type>,
    };

    let mut i: Vec<_> = input.clone().into_iter().collect();
    if let Some(TokenTree::Group(g)) = i.last() {
        let mut s = g.stream();
        let len = i.len();
        s.extend(TokenStream::from(expanded));
        i[len - 1] = TokenTree::Group(Group::new(g.delimiter(), s));
    }
    let mut struct_def = TokenStream::from_iter(i.into_iter());

    let input = parse_macro_input!(input as syn::ItemStruct);
    let struct_name = input.ident;
    let generics = input.generics;
    let expanded = quote!(
        impl #generics #struct_name #generics {
            #[allow(dead_code)]
            fn state_mut(&mut self) -> &mut #state_type {
                self.state.as_mut().expect(&format!("Expected state to exist"))
            }

            #[allow(dead_code)]
            fn state_ref(&self) -> & #state_type {
                self.state.as_ref().expect(&format!("Expected state to exist"))
            }
        }
    );
    struct_def.extend(TokenStream::from(expanded));

    struct_def
}

#[proc_macro_attribute]
pub fn state_component_impl(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as syn::AttributeArgs);
    let state_type = attr.first().unwrap();

    let expanded = quote! {
        fn replace_state(&mut self, other_state: Box<dyn std::any::Any>) {
            if let Ok(s) = other_state.downcast::<#state_type>() {
                self.state = Some(*s);
            }
        }

        fn take_state(&mut self) -> Option<Box<dyn std::any::Any>> {
            if let Some(s) = self.state.take() {
                Some(Box::new(s))
            } else {
                None
            }
        }
    };

    let mut i: Vec<_> = input.clone().into_iter().collect();
    if let Some(TokenTree::Group(g)) = i.last() {
        let mut s = g.stream();
        let len = i.len();
        s.extend(TokenStream::from(expanded));
        i[len - 1] = TokenTree::Group(Group::new(g.delimiter(), s));
    }

    TokenStream::from_iter(i.into_iter())
}
