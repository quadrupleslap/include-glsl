use include_glsl::include_glsl;

static FRAG: &[u8] = include_glsl!("hello.frag");
static VERT: &[u8] = include_glsl!("hello.vert");

fn main() {
    println!("The fragment shader is {} bytes long.", FRAG.len());
    println!("The vertex shader is {} bytes long.", VERT.len());
}
