use std::io;
use std::env::temp_dir;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use assert_cmd::cargo::CommandCargoExt;
use abra_core::lexer;
use abra_core::lexer::tokens::Token;
use abra_core::parser::ast::ModuleId;
use similar::{TextDiff, ChangeTag};
use abra_core::common::display_error::DisplayError;
use abra_core::common::util::{get_project_root, random_string};

enum TestType {
    VsRust(&'static str),
    VsTxt(&'static str, &'static str),
}

pub struct TestRunner {
    bin_path: String,
    tests: Vec<TestType>,
}

impl TestRunner {
    pub fn lexer_test_runner() -> Self {
        let selfhost_dir = get_project_root().unwrap().join("selfhost");
        let build_dir = if let Some(test_temp_dir) = std::env::var("TEST_TMP_DIR").ok() {
            let dir = Path::new(&test_temp_dir).join(random_string(12));
            std::fs::create_dir(&dir).unwrap();
            dir
        } else {
            temp_dir()
        };

        let output = Command::cargo_bin("abra").unwrap()
            .arg("build")
            .arg(&selfhost_dir.join("src/lexer.test.abra"))
            .arg("-o")
            .arg("lexer_test")
            .arg("-b")
            .arg(&build_dir)
            .output()
            .unwrap();
        assert!(output.stderr.is_empty(), "Compilation error: {}", String::from_utf8(output.stderr).unwrap());

        let bin_path = build_dir.join(".abra/lexer_test").to_str().unwrap().to_string();
        Self { bin_path, tests: vec![] }
    }

    pub fn add_test_vs_rust(mut self, test_path: &'static str) -> Self {
        self.tests.push(TestType::VsRust(test_path));
        self
    }

    pub fn add_test_vs_txt(mut self, test_path: &'static str, txt_path: &'static str) -> Self {
        self.tests.push(TestType::VsTxt(test_path, txt_path));
        self
    }

    pub fn run_tests(self) {
        let selfhost_dir = get_project_root().unwrap().join("selfhost");

        let Self { bin_path, tests } = self;

        let mut failures = vec![];
        for test in tests {

            let (test_path, expected_output) = match test {
                TestType::VsRust(test_file_path) => {
                    let test_path = selfhost_dir.join("test").join(test_file_path);
                    let test_path = test_path.to_str().unwrap().to_string();
                    println!("Running lexer test (vs rust) {}", &test_path);

                    let module_id = ModuleId::parse_module_path("./test").unwrap();
                    let contents = std::fs::read_to_string(&test_path).unwrap();
                    let rust_output = match lexer::lexer::tokenize(&module_id, &contents) {
                        Ok(tokens) => tokens_to_json(&tokens).unwrap(),
                        Err(err) => err.get_message(&test_path, &contents)
                    };

                    (test_path, rust_output)
                }
                TestType::VsTxt(test_file_path, comparison_file_path) => {
                    let test_path = selfhost_dir.join("test").join(test_file_path);
                    let test_path = test_path.to_str().unwrap().to_string();
                    println!("Running lexer test (vs file) {}", &test_path);

                    let comparison_path = selfhost_dir.join("test").join(comparison_file_path);
                    let comparison = std::fs::read_to_string(&comparison_path).unwrap();
                    let comparison = comparison.replace("%FILE_NAME%", &test_path);

                    (test_path, comparison)
                }
            };

            let output = Command::new(&bin_path)
                .arg(&test_path)
                .output()
                .unwrap();
            assert!(output.stderr.is_empty(), "Runtime error: {}", String::from_utf8(output.stderr).unwrap());
            let abra_output = String::from_utf8(output.stdout).unwrap();

            if expected_output != abra_output {
                eprintln!("  Difference detected between:");
                eprintln!("    (The expected output is the 'old' and abra output is the 'new')");
                let diff = TextDiff::from_lines(&expected_output, &abra_output);
                for change in diff.iter_all_changes() {
                    let sign = match change.tag() {
                        ChangeTag::Equal => " ",
                        ChangeTag::Delete => "-",
                        ChangeTag::Insert => "+",
                    };
                    eprint!("  {sign}{change}");
                }
                failures.push(test_path);
            }
        }

        if !failures.is_empty() {
            eprintln!("Failures running lexer tests:");
            for test_path in failures {
                eprintln!("  Test path '{}' failed", &test_path)
            }
            panic!("Failures running lexer tests!");
        } else {
            println!("Lexer tests passed!")
        }
    }
}

fn tokens_to_json(tokens: &Vec<Token>) -> io::Result<String> {
    let mut buf = BufWriter::new(Vec::new());

    writeln!(&mut buf, "[")?;
    let len = tokens.len();
    for (idx, token) in tokens.iter().enumerate() {
        writeln!(&mut buf, "  {{")?;
        let pos = token.get_position();
        writeln!(&mut buf, "    \"position\": [{}, {}],", pos.line, pos.col)?;
        writeln!(&mut buf, "    \"kind\": {{")?;
        match token {
            Token::Int(_, val) => {
                writeln!(&mut buf, "      \"name\": \"Int\",")?;
                writeln!(&mut buf, "      \"value\": {}", val)?;
            }
            Token::Float(_, val) => {
                writeln!(&mut buf, "      \"name\": \"Float\",")?;
                writeln!(&mut buf, "      \"value\": {}", val)?;
            }
            Token::String(_, val) => {
                writeln!(&mut buf, "      \"name\": \"String\",")?;
                writeln!(&mut buf, "      \"value\": \"{}\"", val)?;
            }
            Token::StringInterp(_, _) => todo!(),
            Token::Bool(_, _) => todo!(),
            Token::Func(_) => todo!(),
            Token::Val(_) => todo!(),
            Token::Var(_) => todo!(),
            Token::If(_) => todo!(),
            Token::Else(_) => todo!(),
            Token::While(_) => todo!(),
            Token::Break(_) => todo!(),
            Token::Continue(_) => todo!(),
            Token::For(_) => todo!(),
            Token::In(_) => todo!(),
            Token::Match(_) => todo!(),
            Token::Type(_) => todo!(),
            Token::Enum(_) => todo!(),
            Token::Return(_, _) => todo!(),
            Token::Readonly(_) => todo!(),
            Token::Import(_) => todo!(),
            Token::Export(_) => todo!(),
            Token::From(_) => todo!(),
            Token::As(_) => todo!(),
            Token::Try(_) => todo!(),
            Token::Ident(_, name) => {
                writeln!(&mut buf, "      \"name\": \"Ident\",")?;
                writeln!(&mut buf, "      \"value\": \"{}\"", name)?;
            }
            Token::Self_(_) => todo!(),
            Token::None(_) => todo!(),
            Token::Assign(_) => writeln!(&mut buf, "      \"name\": \"Assign\"")?,
            Token::Plus(_) => writeln!(&mut buf, "      \"name\": \"Plus\"")?,
            Token::PlusEq(_) => writeln!(&mut buf, "      \"name\": \"PlusEq\"")?,
            Token::Minus(_) => writeln!(&mut buf, "      \"name\": \"Minus\"")?,
            Token::MinusEq(_) => writeln!(&mut buf, "      \"name\": \"MinusEq\"")?,
            Token::Star(_) => writeln!(&mut buf, "      \"name\": \"Star\"")?,
            Token::StarEq(_) => writeln!(&mut buf, "      \"name\": \"StarEq\"")?,
            Token::Slash(_) => writeln!(&mut buf, "      \"name\": \"Slash\"")?,
            Token::SlashEq(_) => writeln!(&mut buf, "      \"name\": \"SlashEq\"")?,
            Token::Percent(_) => writeln!(&mut buf, "      \"name\": \"Percent\"")?,
            Token::PercentEq(_) => writeln!(&mut buf, "      \"name\": \"PercentEq\"")?,
            Token::And(_) => writeln!(&mut buf, "      \"name\": \"And\"")?,
            Token::AndEq(_) => writeln!(&mut buf, "      \"name\": \"AndEq\"")?,
            Token::Or(_) => writeln!(&mut buf, "      \"name\": \"Or\"")?,
            Token::OrEq(_) => writeln!(&mut buf, "      \"name\": \"OrEq\"")?,
            Token::Caret(_) => writeln!(&mut buf, "      \"name\": \"Caret\"")?,
            Token::Elvis(_) => writeln!(&mut buf, "      \"name\": \"Elvis\"")?,
            Token::ElvisEq(_) => writeln!(&mut buf, "      \"name\": \"ElvisEq\"")?,
            Token::GT(_) => writeln!(&mut buf, "      \"name\": \"GT\"")?,
            Token::GTE(_) => writeln!(&mut buf, "      \"name\": \"GTE\"")?,
            Token::LT(_) => writeln!(&mut buf, "      \"name\": \"LT\"")?,
            Token::LTE(_) => writeln!(&mut buf, "      \"name\": \"LTE\"")?,
            Token::Eq(_) => writeln!(&mut buf, "      \"name\": \"Eq\"")?,
            Token::Neq(_) => writeln!(&mut buf, "      \"name\": \"Neq\"")?,
            Token::Bang(_) => writeln!(&mut buf, "      \"name\": \"Bang\"")?,
            Token::StarStar(_) =>  writeln!(&mut buf, "      \"name\": \"StarStar\"")?,
            Token::LParen(_, _) => writeln!(&mut buf, "      \"name\": \"LParen\"")?,
            Token::RParen(_) => writeln!(&mut buf, "      \"name\": \"RParen\"")?,
            Token::LBrack(_, _) => writeln!(&mut buf, "      \"name\": \"LBrack\"")?,
            Token::RBrack(_) => writeln!(&mut buf, "      \"name\": \"RBrack\"")?,
            Token::LBrace(_) => writeln!(&mut buf, "      \"name\": \"LBrace\"")?,
            Token::RBrace(_) => writeln!(&mut buf, "      \"name\": \"RBrace\"")?,
            Token::LBraceHash(_) => writeln!(&mut buf, "      \"name\": \"LBraceHash\"")?,
            Token::Pipe(_) => writeln!(&mut buf, "      \"name\": \"Pipe\"")?,
            Token::Colon(_) => writeln!(&mut buf, "      \"name\": \"Colon\"")?,
            Token::Comma(_) => writeln!(&mut buf, "      \"name\": \"Comma\"")?,
            Token::Question(_) => writeln!(&mut buf, "      \"name\": \"Question\"")?,
            Token::Dot(_) => writeln!(&mut buf, "      \"name\": \"Dot\"")?,
            Token::QuestionDot(_) => writeln!(&mut buf, "      \"name\": \"QuestionDot\"")?,
            Token::Arrow(_) => writeln!(&mut buf, "      \"name\": \"Arrow\"")?,
            Token::At(_) => writeln!(&mut buf, "      \"name\": \"At\"")?,
        }
        writeln!(&mut buf, "    }}")?;
        write!(&mut buf, "  }}")?;
        if idx != len - 1 {
            writeln!(&mut buf, ",")?;
        } else {
            writeln!(&mut buf, "")?;
        }
    }

    writeln!(&mut buf, "]")?;

    let bytes = buf.into_inner()?;
    Ok(String::from_utf8(bytes).unwrap())
}
