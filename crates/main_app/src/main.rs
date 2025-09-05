// 欢迎使用文件编码转换器！这个文件包含了程序的主要逻辑。
// 考虑到您是Rust新手，我添加了详细的注释来解释每个部分的功能。

// 从外部库（crates）和Rust标准库中导入必要的模块。
// `serde` 用于序列化和反序列化数据。我们使用它的 `Deserialize` 特性。
use serde::Deserialize;
// `encoding_rs` 是处理字符编码的核心库。
use encoding_rs::Encoding;
// `std::fs` 提供了与文件系统交互的功能，比如读写文件。
use std::fs;
// `std::io` 提供了核心的I/O功能。我们使用 `Read` 和 `Write` 这两个trait。
use std::io::{Read, Write};
// `std::path` 用于以跨平台的方式处理文件系统路径。
use std::path::{Path, PathBuf};
// `walkdir` 是一个用于遍历目录树的库。
use walkdir::WalkDir;

// 这个结构体定义了我们的配置文件的结构。
// `#[derive(Deserialize)]` 这个属性来自 `serde` 库。
// 它会自动生成所需的代码，用来将TOML这样的格式解析（反序列化）到这个 `Config` 结构体中。
// 这里的字段名必须和TOML文件中的键（key）完全匹配。
#[derive(Deserialize, Debug)]
struct Config {
    path: String,
    target_encoding: String,
    file_extension: String,
}

// `main` 函数是每个Rust可执行程序的入口点。
fn main() {
    println!("启动文件编码转换器...");

    // 定义配置文件的名称。
    let config_filename = "config.toml";

    // 将配置文件的内容读取到一个字符串中。
    // `fs::read_to_string` 返回一个 `Result` 类型。我们使用 `expect` 来处理文件无法读取的情况。
    // 如果失败，程序会panic（崩溃）并显示我们提供的消息。
    // 当你预期在正常操作中不会发生错误时，这是一种简单的错误处理方式。
    let config_content = fs::read_to_string(config_filename)
        .expect(&format!("错误：无法读取配置文件: {}", config_filename));

    // 将TOML内容解析到我们的 `Config` 结构体中。
    // `toml::from_str` 同样返回一个 `Result`。这里，我们使用 `match` 语句来更优雅地处理它。
    let config: Config = match toml::from_str(&config_content) {
        // 如果解析成功，会返回 `Ok(parsed_config)`，我们使用这个解析后的配置。
        Ok(parsed_config) => parsed_config,
        // 如果解析失败，会返回 `Err(e)`。我们打印出错误信息并退出程序。
        Err(e) => {
            eprintln!("错误：解析配置文件失败: \n详情: {}", e);
            // 使用非零状态码退出，表示程序发生了错误。
            std::process::exit(1);
        }
    };

    println!("配置加载成功: {:?}", config);

    // 从配置中提供的字符串标签（如"utf-8"）获取一个 `Encoding` 对象。
    let target_encoding =
        Encoding::for_label(config.target_encoding.as_bytes()).unwrap_or_else(|| {
            // 如果编码标签无效，打印一个错误并默认使用UTF-8。
            eprintln!(
                "错误：无效的目标编码 '{}'。将默认使用UTF-8。",
                config.target_encoding
            );
            encoding_rs::UTF_8
        });

    // 将配置中的字符串路径转换为 `PathBuf` 对象，以便于处理。
    let path = PathBuf::from(config.path);
    if !path.exists() {
        eprintln!("错误：配置文件中指定的路径不存在: {}", path.display());
        return; // 提前退出main函数。
    }

    // 检查路径是目录还是文件，并调用相应的处理函数。
    if path.is_dir() {
        println!("路径是一个目录。正在处理...");
        process_directory(&path, &config.file_extension, target_encoding);
    } else if path.is_file() {
        println!("路径是一个文件。正在处理...");
        match process_file(&path, target_encoding) {
            Ok(_) => println!("成功转换文件: {}", path.display()),
            Err(e) => eprintln!("转换文件 {} 时出错: {}", path.display(), e),
        }
    }
    println!("转换过程结束。");

    // 添加以下代码来等待用户输入
    println!("按回车键退出...");
    let mut buffer = String::new();
    // 读取用户在命令行的一行输入
    // 如果读取失败，.unwrap()会让程序panic，对于这个简单场景是可接受的
    std::io::stdin().read_line(&mut buffer).unwrap();
}

// 这个函数处理指定目录下的所有相关文件。
// 它接收目录 `path`、用于过滤的 `ext` 后缀名和 `target_encoding` 目标编码。
fn process_directory(path: &Path, ext: &str, target_encoding: &'static Encoding) {
    // `WalkDir::new(path)` 创建一个迭代器，它会递归地遍历目录。
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        // 我们对目录中的每个条目检查三个条件：
        // 1. 它是一个文件吗？
        // 2. 它有文件后缀名吗？
        // 3. 这个后缀名和我们配置中的匹配吗？
        if entry.path().is_file() && entry.path().extension().map_or(false, |e| e == ext) {
            // 如果所有条件都满足，我们就处理这个文件。
            // 我们使用 `match` 来处理 `process_file` 函数返回的 `Result`。
            match process_file(entry.path(), target_encoding) {
                Ok(_) => println!("  -> 成功转换: {}", entry.path().display()),
                Err(e) => eprintln!("  -> 转换 {} 时出错: {}", entry.path().display(), e),
            }
        }
    }
}

// 这个函数处理单个文件的转换逻辑。
// 它返回一个 `Result<(), Box<dyn std::error::Error>>` 类型。
// - `()` 表示如果成功，它不返回任何有意义的值（就是“OK”的意思）。
// - `Box<dyn std::error::Error>` 是一个通用的错误类型。它表示如果失败，
//   函数可以返回任何类型的错误，这让错误处理变得非常灵活。
fn process_file(
    file_path: &Path,
    target_encoding: &'static Encoding,
) -> Result<(), Box<dyn std::error::Error>> {
    // --- 第1步: 将整个文件读取到一个字节向量 (`Vec<u8>`) 中。
    let mut file = fs::File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    // `?` 操作符是错误处理的简写。如果 `fs::File::open` 或 `read_to_end`
    // 返回一个 `Err`，`?` 会立即从当前函数返回那个错误。

    // --- 第2步: 自动检测源文件的编码。
    // BOM (字节顺序标记) 是文件开头的一个特殊字节序列，用于表明文件的编码（如UTF-8, UTF-16等）。
    // `Encoding::for_bom` 会检查这个标记。
    // 如果没有找到BOM，它返回 `None`，我们使用 `unwrap_or` 来默认使用 `WINDOWS_1252`，
    // 这是一个非常常见的传统编码。
    let (source_encoding, bom_len) =
        Encoding::for_bom(&buffer).unwrap_or((encoding_rs::WINDOWS_1252, 0));
    println!(
        "    - 检测到编码: {} 位于 {}",
        source_encoding.name(),
        file_path.display()
    );

    // --- 第3步: 将字节解码成Rust的String类型。
    // Rust的 `String` 类型内部总是使用UTF-8编码。这一步将原始文件内容
    // (例如，从WINDOWS-1252编码) 转换为这种标准的内部格式。
    // 我们从BOM之后（如果存在BOM的话）的第一个字节开始解码。
    let (decoded_str, _, had_errors) = source_encoding.decode(&buffer[bom_len..]);
    if had_errors {
        println!("    - 警告: 解码时遇到错误。某些字符可能没有被正确转换。");
    }

    // --- 第4步: 将Rust的String编码为目标编码的字节格式。
    // 现在我们获取标准的内部 `String`，并将其转换为我们期望的目标编码的字节序列。
    let (encoded_bytes, _, had_errors) = target_encoding.encode(&decoded_str);
    if had_errors {
        // 这是一个更严重的错误，意味着我们无法在目标编码中表示文件内容。
        // 我们返回一个错误来停止对这个文件的处理。
        return Err(format!("无法编码到 {}", target_encoding.name()).into());
    }

    // --- 第5步: 将新编码的字节写回文件。
    // `fs::File::create` 会打开一个文件用于写入。如果文件已存在，它的内容
    // 会被清空（截断）。这就是我们覆盖原始文件的方式。
    let mut file = fs::File::create(file_path)?;
    file.write_all(&encoded_bytes)?;
    println!("    - 文件已成功用 {} 编码覆盖。", target_encoding.name());

    // 如果所有步骤都顺利完成，没有触发 `?` 或显式的 `return Err`，
    // 函数会隐式地返回 `Ok(())` 来表示成功。
    Ok(())
}
