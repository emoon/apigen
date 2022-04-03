[![Build Status](https://github.com/emoon/apigen/workflows/Rust/badge.svg)](https://github.com/emoon/apigen/actions?workflow=Rust)

# apigen
Basic API/data description lang + parser that can be used for generating API/data interfaces, etc

It allows you to have structs like this

```Rust
struct ImageInfo {
    /// width of the image
    width: u32,
    /// height of the Image
    height: u32,
}

#[attributes(Handle, Drop)]
struct Image {
    /// Comments here 
    [static] create_from_file(filename: String) -> Image?,
    /// More comments 
    [static] create_from_memory(name: String, data: [u8]) -> Image?,
    /// Get data amout the image
    [static] get_info(image: Image) -> *const ImageInfo?,
    /// Destroy the created image
    destroy(),
}
```

And it gets parsed into data structures. It's then up to the user to decide how to to write this data out. Some convinince functionally for C and Rust is provided as that is the primary target of this. The full grammar for this can be found here https://github.com/emoon/apigen/blob/main/src/api.pest