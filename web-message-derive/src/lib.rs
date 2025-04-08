use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Expr, ExprLit, Fields, Lit, Meta, MetaNameValue, parse_macro_input};

#[proc_macro_derive(Message, attributes(msg))]
pub fn derive_message(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = &input.ident;

	let output = match &input.data {
		syn::Data::Struct(data) => expand_struct(ident, &data.fields),
		syn::Data::Enum(data_enum) => {
			let tag_field = get_tag_field(&input.attrs).unwrap_or_else(|| {
				panic!("#[derive(Message)] on enums requires #[msg(tag = \"type\")]");
			});

			expand_enum(ident, &tag_field, &data_enum.variants)
		}
		_ => panic!("Message only supports structs and enums"),
	};

	output.into()
}

fn get_tag_field(attrs: &[Attribute]) -> Option<String> {
	for attr in attrs {
		if attr.path().is_ident("msg") {
			if let Ok(Meta::NameValue(MetaNameValue { path, value, .. })) = attr.parse_args() {
				if path.is_ident("tag") {
					if let Expr::Lit(ExprLit {
						lit: Lit::Str(lit_str), ..
					}) = value
					{
						return Some(lit_str.value());
					}
				}
			}
		}
	}
	None
}

fn expand_struct(ident: &syn::Ident, fields: &Fields) -> proc_macro2::TokenStream {
	let field_inits = fields.iter().map(|f| {
		let name = f.ident.as_ref().unwrap();
		let name_str = name.to_string();

		let is_transferable = f.attrs.iter().any(|attr| {
			attr.path().is_ident("msg")
				&& attr
					.parse_args::<syn::Ident>()
					.map_or(false, |ident| ident == "transferable")
		});

		if is_transferable {
			quote! {
				#name: ::js_sys::Reflect::get(&obj, &#name_str.into()).map_err(|_| ::web_message::Error::MissingField(#name_str))?
					.into()
			}
		} else {
			quote! {
				#name: ::js_sys::Reflect::get(&obj, &#name_str.into()).map_err(|_| ::web_message::Error::MissingField(#name_str))?
					.try_into()
					.map_err(|_| ::web_message::Error::InvalidField(#name_str, ::js_sys::Reflect::get(&obj, &#name_str.into()).unwrap()))?
			}
		}
	});

	let field_assignments = fields.iter().map(|f| {
		let name = f.ident.as_ref().unwrap();
		let name_str = name.to_string();

		let is_transferable = f.attrs.iter().any(|attr| {
			attr.path().is_ident("msg")
				&& attr
					.parse_args::<syn::Ident>()
					.map_or(false, |ident| ident == "transferable")
		});

		if is_transferable {
			quote! {
				::js_sys::Reflect::set(&obj, &#name_str.into(), &self.#name.clone().into()).unwrap();
				transferable.push(&self.#name.into());
			}
		} else {
			quote! {
				::js_sys::Reflect::set(&obj, &#name_str.into(), &self.#name.into()).unwrap();
			}
		}
	});

	quote! {
		impl #ident {
			pub fn from_message(message: ::js_sys::wasm_bindgen::JsValue) -> Result<Self, ::web_message::Error> {
				let obj = js_sys::Object::try_from(&message).ok_or(::web_message::Error::ExpectedObject(message.clone()))?;
				Ok(Self {
					#(#field_inits),*
				})
			}

			pub fn into_message(self) -> (::js_sys::Object, ::js_sys::Array) {
				let obj = ::js_sys::Object::new();
				let transferable = ::js_sys::Array::new();
				#(#field_assignments)*
				(obj, transferable)
			}
		}
	}
}

fn expand_enum(
	enum_ident: &syn::Ident,
	tag_field: &str,
	variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
) -> proc_macro2::TokenStream {
	let from_matches = variants.iter().map(|variant| {
		let variant_ident = &variant.ident;
		let variant_str = variant_ident.to_string();

		match &variant.fields {
			Fields::Named(fields_named) => {
				let field_assignments = fields_named.named.iter().map(|f| {
					let name = f.ident.as_ref().unwrap();
					let name_str = name.to_string();

					let is_transferable = f.attrs.iter().any(|attr| {
						attr.path().is_ident("post")
							&& attr
								.parse_args::<syn::Ident>()
								.map_or(false, |ident| ident == "transferable")
					});

					if is_transferable {
						quote! {
							#name: ::js_sys::Reflect::get(&obj, &#name_str.into()).map_err(|_| ::web_message::Error::MissingField(#name_str))?
								.into()
						}
					} else {
						quote! {
							#name: ::js_sys::Reflect::get(&obj, &#name_str.into()).map_err(|_| ::web_message::Error::MissingField(#name_str))?
								.try_into()
								.map_err(|_| ::web_message::Error::InvalidField(#name_str, ::js_sys::Reflect::get(&obj, &#name_str.into()).unwrap()))?
						}
					}
				});

				quote! {
					#variant_str => {
						Ok(#enum_ident::#variant_ident {
							#(#field_assignments),*
						})
					}
				}
			}

			Fields::Unit => {
				quote! {
					#variant_str => Ok(#enum_ident::#variant_ident),
				}
			}

			_ => unimplemented!("web-message does not support tuple variants (yet)"),
		}
	});

	let into_matches = variants.iter().map(|variant| {
		let variant_ident = &variant.ident;
		let variant_str = variant_ident.to_string();

		match &variant.fields {
			Fields::Named(fields_named) => {
				let field_names = fields_named.named.iter().map(|f| f.ident.as_ref().unwrap());

				let set_fields = fields_named.named.iter().map(|f| {
					let name = f.ident.as_ref().unwrap();
					let name_str = name.to_string();

					let is_transferable = f.attrs.iter().any(|attr| {
						attr.path().is_ident("post")
							&& attr
								.parse_args::<syn::Ident>()
								.map_or(false, |ident| ident == "transferable")
					});

					if is_transferable {
						quote! {
							::js_sys::Reflect::set(&obj, &#name_str.into(), &#name.clone().into()).unwrap();
							transferable.push(&#name.into());
						}
					} else {
						quote! {
							::js_sys::Reflect::set(&obj, &#name_str.into(), &#name.into()).unwrap();
						}
					}
				});

				quote! {
					#enum_ident::#variant_ident { #(#field_names),* } => {
						::js_sys::Reflect::set(&obj, &#tag_field.into(), &#variant_str.into()).unwrap();
						#(#set_fields)*
					}
				}
			}
			Fields::Unit => {
				quote! {
					#enum_ident::#variant_ident => {
						::js_sys::Reflect::set(&obj, &#tag_field.into(), &#variant_str.into()).unwrap();
					}
				}
			}
			_ => unimplemented!("web-message does not support tuple variants (yet)"),
		}
	});

	quote! {
		impl #enum_ident {
			pub fn from_message(message: ::js_sys::wasm_bindgen::JsValue) -> Result<Self, ::web_message::Error> {
				let obj = js_sys::Object::try_from(&message).ok_or(::web_message::Error::ExpectedObject(message.clone()))?;
				let tag_val = ::js_sys::Reflect::get(&obj, &#tag_field.into()).map_err(|_| ::web_message::Error::MissingTag(#tag_field))?;
				let tag_str = tag_val.as_string()
					.ok_or(::web_message::Error::InvalidTag(#tag_field, tag_val.clone()))?;

				match tag_str.as_str() {
					#(#from_matches)*
					_ => Err(::web_message::Error::UnknownTag(#tag_field, tag_val.clone())),
				}
			}

			pub fn into_message(self) -> (::js_sys::Object, ::js_sys::Array) {
				let obj = ::js_sys::Object::new();
				let transferable = ::js_sys::Array::new();

				match self {
					#(#into_matches),*
				}

				(obj, transferable)
			}
		}
	}
}
