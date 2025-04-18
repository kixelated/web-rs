use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Message, attributes(msg))]
pub fn derive_message(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = &input.ident;

	let output = match &input.data {
		syn::Data::Struct(data) => expand_struct(ident, &data.fields),
		syn::Data::Enum(data_enum) => expand_enum(ident, &data_enum.variants),
		_ => panic!("Message only supports structs and enums (for now)"),
	};

	output.into()
}

fn expand_struct(ident: &syn::Ident, fields: &Fields) -> proc_macro2::TokenStream {
	let field_inits = fields.iter().map(|f| {
		let name = f.ident.as_ref().unwrap();
		let name_str = name.to_string();

		quote! {
			#name: ::web_message::Message::from_message(::web_sys::js_sys::Reflect::get(&obj, &#name_str.into()).map_err(|_| ::web_message::Error::MissingField(#name_str))?)
				.map_err(|_| ::web_message::Error::InvalidField(#name_str))?
		}
	});

	let field_assignments = fields.iter().map(|f| {
		let name = f.ident.as_ref().unwrap();
		let name_str = name.to_string();

		quote! {
			::web_sys::js_sys::Reflect::set(&obj, &#name_str.into(), &self.#name.into()).unwrap();
		}
	});

	quote! {
		impl ::web_message::Message for #ident {
			fn from_message(message: ::web_sys::js_sys::wasm_bindgen::JsValue) -> Result<Self, ::web_message::Error> {
				let obj = web_sys::js_sys::Object::try_from(&message).ok_or(::web_message::Error::UnexpectedType)?;
				Ok(Self {
					#(#field_inits),*
				})
			}

			fn into_message(self, _transferable: &mut ::web_sys::js_sys::Array) -> ::web_sys::js_sys::wasm_bindgen::JsValue {
				let obj = ::web_sys::js_sys::Object::new();
				#(#field_assignments)*
				obj.into()
			}
		}
	}
}

fn expand_enum(
	enum_ident: &syn::Ident,
	variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
) -> proc_macro2::TokenStream {
	let from_obj= variants.iter().filter_map(|variant| {
		let variant_ident = &variant.ident;
		let variant_str = variant_ident.to_string();

		match &variant.fields {
			Fields::Named(fields_named) => {
				let field_assignments = fields_named.named.iter().map(|f| {
					let name = f.ident.as_ref().unwrap();
					let name_str = name.to_string();

					quote! {
						#name: ::web_message::Message::from_message(::web_sys::js_sys::Reflect::get(&val, &#name_str.into()).map_err(|_| ::web_message::Error::MissingField(#name_str))?)
							.map_err(|_| ::web_message::Error::InvalidField(#name_str))?
					}
				});

				Some(quote! {
					#variant_str if val.is_object() => {
						Ok(#enum_ident::#variant_ident {
							#(#field_assignments),*
						})
					}
					#variant_str => Err(::web_message::Error::UnexpectedType),
				})
			}

			Fields::Unit => None,

			Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => {
				Some(quote! {
					#variant_str => Ok(#enum_ident::#variant_ident(::web_message::Message::from_message(val)?)),
				})
			},

			Fields::Unnamed(fields_unnamed) => {
				let fields_count = fields_unnamed.unnamed.len() as u32;
				let field_assignments = (0..fields_count).map(|i| {
					quote! {
						::web_message::Message::from_message(arr.get(#i))
							.map_err(|_| ::web_message::Error::InvalidField(stringify!(#i)))?
					}
				});

				Some(quote! {
					#variant_str if val.is_array() => {
						let arr = ::web_sys::js_sys::Array::from(&val);
						if arr.length() != #fields_count {
							return Err(::web_message::Error::UnexpectedLength);
						}

						Ok(#enum_ident::#variant_ident (
							#(#field_assignments),*
						))
					}
					#variant_str => Err(::web_message::Error::UnexpectedType),
				})
			}
		}
	});

	let from_string = variants.iter().filter_map(|variant| {
		let variant_ident = &variant.ident;
		let variant_str = variant_ident.to_string();

		if let Fields::Unit = &variant.fields {
			Some(quote! {
				#variant_str => Ok(#enum_ident::#variant_ident),
			})
		} else {
			None
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

					quote! {
						::web_sys::js_sys::Reflect::set(&inner, &#name_str.into(), &#name.into_message(_transferable)).unwrap();
					}
				});

				quote! {
					#enum_ident::#variant_ident { #(#field_names),* } => {
						let obj = ::web_sys::js_sys::Object::new();
						let inner = ::web_sys::js_sys::Object::new();
						#(#set_fields)*
						::web_sys::js_sys::Reflect::set(&obj, &#variant_str.into(), &inner.into()).unwrap();
						obj.into()
					}
				}
			}
			Fields::Unit => {
				quote! {
					#enum_ident::#variant_ident => #variant_str.into()
				}
			}
			Fields::Unnamed(fields_unnamed) if fields_unnamed.unnamed.len() == 1 => {
				quote! {
					#enum_ident::#variant_ident(v) => {
						let obj = ::web_sys::js_sys::Object::new();
						::web_sys::js_sys::Reflect::set(&obj, &#variant_str.into(), &v.into_message(_transferable)).unwrap();
						obj.into()
					}
				}
			}
			Fields::Unnamed(fields_unnamed) => {
				let fields_count = fields_unnamed.unnamed.len();
				let field_idents: Vec<_> = (0..fields_count)
					.map(|i| syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()))
					.collect();

				let set_fields = field_idents.iter().map(|f| {
					quote! {
						inner.push(&#f.into_message(_transferable));
					}
				});

				quote! {
					#enum_ident::#variant_ident(#(#field_idents),*) => {
						let obj = ::web_sys::js_sys::Object::new();
						let inner = ::web_sys::js_sys::Array::new();
						#(#set_fields)*
						::web_sys::js_sys::Reflect::set(&obj, &#variant_str.into(), &inner.into()).unwrap();
						obj.into()
					}
				}
			}
		}
	});

	quote! {
		impl ::web_message::Message for #enum_ident {
			fn from_message(message: ::web_sys::js_sys::wasm_bindgen::JsValue) -> ::std::result::Result<Self, ::web_message::Error> {
				if let Some(s) = message.as_string() {
					match s.as_str() {
						#(#from_string)*
						_ => Err(::web_message::Error::UnknownTag),
					}
				} else if let Some(obj) = web_sys::js_sys::Object::try_from(&message) {
					let keys = web_sys::js_sys::Object::keys(&obj);
					if keys.length() != 1 {
						return Err(::web_message::Error::UnexpectedType);
					}

					let tag = keys.get(0);
					let tag_str = tag.as_string().ok_or(::web_message::Error::UnexpectedType)?;

					let val = ::web_sys::js_sys::Reflect::get(&obj, &tag).unwrap();

					match tag_str.as_str() {
						#(#from_obj)*
						_ => Err(::web_message::Error::UnknownTag),
					}
				} else {
					Err(::web_message::Error::UnexpectedType)
				}
			}

			fn into_message(self, _transferable: &mut ::web_sys::js_sys::Array) -> ::web_sys::js_sys::wasm_bindgen::JsValue {
				match self {
					#(#into_matches),*
				}
			}
		}
	}
}
