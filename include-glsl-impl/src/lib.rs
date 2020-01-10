#![feature(proc_macro_span)]

use proc_macro2::{TokenStream, Literal};
use proc_macro_hack::proc_macro_hack;
use quote::quote;
use shaderc::{Compiler, CompileOptions, ShaderKind, OptimizationLevel};
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

macro_rules! tr {
    ($span:expr, $x:expr) => {
        match $x {
            Ok(x) => x,
            Err(e) => return syn::Error::new($span.into(), e).to_compile_error().into(),
        }
    }
}

thread_local! {
    static COMPILER: RefCell<Option<Compiler>> = RefCell::new(None);
}

fn get_compiler() -> Option<Compiler> {
    COMPILER.with(|c| c.borrow_mut().take()).or_else(Compiler::new)
}

fn put_compiler(compiler: Compiler) -> Option<Compiler> {
    COMPILER.with(|c| c.replace(Some(compiler)))
}

#[proc_macro_hack]
pub fn include_glsl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let relative_path = {
        let input = input.clone();
        PathBuf::from(syn::parse_macro_input!(input as syn::LitStr).value())
    };

    let span = input.clone().into_iter().next().unwrap().span();

    let path = if relative_path.is_absolute() {
        relative_path
    } else {
        let source_file = span.source_file();

        if !source_file.is_real() {
            tr!(span, Err("unknown base for relative path"));
        }

        let mut path = source_file.path();
        path.pop();
        path.push(relative_path);
        path
    };

    let mut file = tr!(span, File::open(&path));
    let mut glsl = String::new();
    tr!(span, file.read_to_string(&mut glsl));

    let mut compiler = tr!(span, get_compiler().ok_or("could not initialize the GLSL compiler"));
    let mut compile_options = tr!(span, CompileOptions::new().ok_or("could not initialize the GLSL compiler options object"));
    compile_options.set_warnings_as_errors();
    compile_options.set_optimization_level(OptimizationLevel::Performance);

    let shader_kind = match path.extension() {
        Some(x) if x == "vert" => ShaderKind::DefaultVertex,
        Some(x) if x == "frag" => ShaderKind::DefaultFragment,
        Some(x) if x == "comp" => ShaderKind::DefaultCompute,
        Some(x) if x == "geom" => ShaderKind::DefaultGeometry,
        Some(x) if x == "tesc" => ShaderKind::DefaultTessControl,
        Some(x) if x == "tese" => ShaderKind::DefaultTessEvaluation,
        _ => ShaderKind::InferFromSource,
    };

    let entry_point_name = "main";
    let input_file_name = path
        .file_name()
        .map(|x| x.to_string_lossy())
        .unwrap_or_else(|| "<unknown>".into());

    let result = tr!(span, compiler.compile_into_spirv(
        &glsl,
        shader_kind,
        &input_file_name,
        entry_point_name,
        Some(&compile_options),
    ));

    put_compiler(compiler);

    let input = TokenStream::from(input);
    let bytes = Literal::byte_string(result.as_binary_u8());
    proc_macro::TokenStream::from(quote! {
        { ::core::include_bytes!(#input); #bytes }
    })
}
