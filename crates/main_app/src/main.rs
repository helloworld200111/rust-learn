// 欢迎使用文件编码转换器！这个文件包含了程序的主要逻辑。
// 考虑到您是Rust新手，我添加了详细的注释来解释每个部分的功能。

// 从外部库（crates）和Rust标准库中导入必要的模块。
use chardet::detect;
use crossterm::{event, terminal};
use encoding_rs::{Encoding, UTF_8};
use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use walkdir::WalkDir;

// 这个结构体定义了我们的配置文件的结构。
// `#[derive(Deserialize)]` 这个属性来自 `serde` 库。
// 它会自动生成所需的代码，用来将TOML这样的格式解析（反序列化）到这个 `Config` 结构体中。
#[derive(Deserialize, Debug)]
struct Config {
    path: String,
    file_extension: String,
    output_encoding: String,
    input_encodings: Vec<String>,
}

// `main` 函数是每个Rust可执行程序的入口点。
fn main() {
    println!("启动文件编码转换器...");

    let config_filename = "config.toml";
    let config_content = match fs::read_to_string(config_filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("错误：无法读取配置文件 '{}': {}", config_filename, e);
            exit(1);
        }
    };

    let config: Config = match toml::from_str(&config_content) {
        Ok(parsed_config) => parsed_config,
        Err(e) => {
            eprintln!(
                "错误：解析配置文件失败: 
详情: {}",
                e
            );
            exit(1);
        }
    };

    println!("配置加载成功: {:?}", config);

    let output_encoding =
        Encoding::for_label(config.output_encoding.as_bytes()).unwrap_or_else(|| {
            eprintln!(
                "错误：无效的目标编码 '{}'。将默认使用UTF-8。",
                config.output_encoding
            );
            UTF_8
        });

    let path = PathBuf::from(config.path);
    if !path.exists() {
        eprintln!("错误：配置文件中指定的路径不存在: {}", path.display());
        exit(1);
    }

    if path.is_dir() {
        println!("路径是一个目录。正在处理...");
        process_directory(
            &path,
            &config.file_extension,
            output_encoding,
            &config.input_encodings,
        );
    } else if path.is_file() {
        println!("路径是一个文件。正在处理...");
        match process_file(&path, output_encoding, &config.input_encodings) {
            Ok(_) => println!("成功转换文件: {}", path.display()),
            Err(e) => {
                eprintln!("转换文件 {} 时出错: {}", path.display(), e);
                exit(1);
            }
        }
    }
    println!("转换过程结束。");
    exit(0);
}

fn exit(code: i32) -> ! {
    println!("按任意键退出...");
    terminal::enable_raw_mode().unwrap();
    loop {
        if let Ok(event::Event::Key(_)) = event::read() {
            break;
        }
    }
    terminal::disable_raw_mode().unwrap();
    process::exit(code);
}

// 处理目录
fn process_directory(
    path: &Path,
    ext: &str,
    output_encoding: &'static Encoding,
    input_encodings: &[String],
) {
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().is_file() && entry.path().extension().map_or(false, |e| e == ext) {
            match process_file(entry.path(), output_encoding, input_encodings) {
                Ok(_) => println!("  -> 成功转换: {}", entry.path().display()),
                Err(e) => eprintln!("  -> 转换 {} 时出错: {}", entry.path().display(), e),
            }
        }
    }
}

// 尝试自动检测编码
fn detect_encoding(buffer: &[u8]) -> Option<(&'static Encoding, usize)> {
    // 1. 检查BOM
    if let Some((encoding, bom_len)) = Encoding::for_bom(buffer) {
        return Some((encoding, bom_len));
    }
    // 2. 使用 chardet 猜测
    let result = detect(buffer);
    Encoding::for_label(result.0.as_bytes()).map(|encoding| (encoding, 0))
}

// 处理单个文件
fn process_file(
    file_path: &Path,
    output_encoding: &'static Encoding,
    input_encodings: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();
    fs::File::open(file_path)?.read_to_end(&mut buffer)?;

    let mut source_encoding_opt: Option<(&'static Encoding, usize)> = None;

    // --- 第1步: 尝试自动检测编码
    if let Some((encoding, bom_len)) = detect_encoding(&buffer) {
        println!(
            "    - 自动检测到编码: {} 位于 {}",
            encoding.name(),
            file_path.display()
        );
        source_encoding_opt = Some((encoding, bom_len));
    } else {
        println!("    - 自动检测失败。尝试配置文件中的备选编码...");
    }

    // --- 第2步: 如果自动检测失败，则尝试配置文件中的编码列表
    let (decoded_str, used_encoding_name) = if let Some((encoding, bom_len)) = source_encoding_opt {
        let (decoded, _, _) = encoding.decode(&buffer[bom_len..]);
        (decoded, encoding.name().to_string())
    } else {
        let mut decoded_result: Option<(String, String)> = None;
        for encoding_name in input_encodings {
            println!("    - 尝试使用备选编码 '{}' 解码……", encoding_name);
            if let Some(encoding) = Encoding::for_label(encoding_name.as_bytes()) {
                let (decoded, _, had_errors) = encoding.decode(&buffer);
                if !had_errors {
                    println!("    - 成功使用备选编码 '{}' 解码", encoding_name);
                    decoded_result = Some((decoded.to_string(), encoding_name.clone()));
                    break;
                }
            }
        }
        match decoded_result {
            Some((s, name)) => (s.into(), name),
            None => {
                return Err(
                    format!("所有备选编码都无法成功解码文件: {}", file_path.display()).into(),
                );
            }
        }
    };

    println!("    - 使用编码 '{}' 进行转换。", used_encoding_name);

    // --- 第3步: 将解码后的字符串用目标编码重新编码
    let (encoded_bytes, _, had_errors) = output_encoding.encode(&decoded_str);
    if had_errors {
        return Err(format!("无法将内容编码到 '{}'", output_encoding.name()).into());
    }

    // --- 第4步: 将新编码的字节写回文件
    fs::File::create(file_path)?.write_all(&encoded_bytes)?;
    println!("    - 文件已成功用 {} 编码覆盖。", output_encoding.name());

    Ok(())
}
