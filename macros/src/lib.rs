use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, ItemStruct, parse_macro_input, spanned::Spanned};

/// Attribute macro `#[take_resources]` applied to a struct.
///
/// This macro generates a `macro_rules!` macro for the struct that allows
/// constructing an instance from a provider struct `$p` where the fields
/// are taken from `$p` in uppercase form.
///
/// # Rules
/// - Only works with structs that have **named fields**.
/// - Converts each field name from `snake_case` to `UPPERCASE` to access `$p`'s fields.
/// - The generated macro is named `take_<struct_name_in_snake_case>`.
///
/// # Example
///
/// ```rust
/// #[take_resources]
/// pub struct BleResources<'d> {
///     pub rtc0: Peri<'d, RTC0>,
///     pub timer0: Peri<'d, TIMER0>,
///     pub temp: Peri<'d, TEMP>,
///     pub rng: Peri<'d, RNG>,
///     pub ppi_ch17: Peri<'d, PPI_CH17>,
///     pub ppi_ch18: Peri<'d, PPI_CH18>,
///     pub ppi_ch19: Peri<'d, PPI_CH19>,
///     pub ppi_ch20: Peri<'d, PPI_CH20>,
///     pub ppi_ch21: Peri<'d, PPI_CH21>,
///     pub ppi_ch22: Peri<'d, PPI_CH22>,
///     pub ppi_ch23: Peri<'d, PPI_CH23>,
///     pub ppi_ch24: Peri<'d, PPI_CH24>,
///     pub ppi_ch25: Peri<'d, PPI_CH25>,
///     pub ppi_ch26: Peri<'d, PPI_CH26>,
///     pub ppi_ch27: Peri<'d, PPI_CH27>,
///     pub ppi_ch28: Peri<'d, PPI_CH28>,
///     pub ppi_ch29: Peri<'d, PPI_CH29>,
///     pub ppi_ch30: Peri<'d, PPI_CH30>,
///     pub ppi_ch31: Peri<'d, PPI_CH31>,
/// }
///
/// // This generates the following macro:
///
/// macro_rules! take_ble_resources {
///     ($p:ident) => {
///         BleResources {
///             rtc0: $p.RTC0,
///             timer0: $p.TIMER0,
///             temp: $p.TEMP,
///             rng: $p.RNG,
///             ppi_ch17: $p.PPI_CH17,
///             ppi_ch18: $p.PPI_CH18,
///             ppi_ch19: $p.PPI_CH19,
///             ppi_ch20: $p.PPI_CH20,
///             ppi_ch21: $p.PPI_CH21,
///             ppi_ch22: $p.PPI_CH22,
///             ppi_ch23: $p.PPI_CH23,
///             ppi_ch24: $p.PPI_CH24,
///             ppi_ch25: $p.PPI_CH25,
///             ppi_ch26: $p.PPI_CH26,
///             ppi_ch27: $p.PPI_CH27,
///             ppi_ch28: $p.PPI_CH28,
///             ppi_ch29: $p.PPI_CH29,
///             ppi_ch30: $p.PPI_CH30,
///             ppi_ch31: $p.PPI_CH31,
///         }
///     };
/// }
/// ```

#[proc_macro_attribute]
pub fn take_resources(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the struct
    let input_struct = parse_macro_input!(item as ItemStruct);
    let struct_name = &input_struct.ident;

    // Collect fields
    let fields = match &input_struct.fields {
        syn::Fields::Named(fields_named) => &fields_named.named,
        _ => panic!("#[take_resources] only works with named struct fields"),
    };

    // Generate macro fields: snake_case -> UPPERCASE
    let macro_fields = fields.iter().map(|f| {
        let field_name = &f.ident;
        let ident_str = field_name.as_ref().unwrap().to_string();

        // Convert snake_case to UPPERCASE (e.g., ppi_ch17 -> PPI_CH17)
        let macro_ident_str = ident_str.to_ascii_uppercase();
        let macro_ident = Ident::new(&macro_ident_str, field_name.span());
        quote! {
            #field_name: $p.#macro_ident
        }
    });

    // Generate macro_rules!
    let macro_name = Ident::new(
        &format!("take_{}", pascal_to_snake(&struct_name.to_string())),
        struct_name.span(),
    );

    let expanded = quote! {
        #input_struct

        #[macro_export]
        macro_rules! #macro_name {
            ($p:ident) => {
                #struct_name {
                    #(#macro_fields),*
                }
            };
        }
    };
    TokenStream::from(expanded)
}

fn pascal_to_snake(name: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                snake.push('_');
            }
            snake.push(ch.to_ascii_lowercase());
        } else {
            snake.push(ch);
        }
    }
    snake
}

