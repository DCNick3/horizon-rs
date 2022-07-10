use crate::swipc::codegen::{gen_ipc_file, TokenStorage};
use crate::swipc::diagnostics::{diagnostics_from_parse_error, DiagnosticResultExt};
use crate::swipc::model::{IpcFile, TypecheckedIpcFile};
use crate::swipc::parser::IpcFileParser;
use anyhow::{anyhow, Context};
use codespan_reporting::term::termcolor::ColorChoice;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::ReadDir;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[derive(clap::Args, Debug)]
pub struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    GenIpcdef {},
}

struct Paths {
    defs_directory: PathBuf,
    gen_directory: PathBuf,
}

fn get_paths() -> anyhow::Result<Paths> {
    let cwd = std::env::current_dir()?;

    // Try to check that our cwd is a root of workspace, just to be safe
    let workspace_toml = cwd.join("Cargo.toml");
    let workspace_toml: toml::Value = toml::from_str(
        &std::fs::read_to_string(workspace_toml).context("Reading root workspace toml")?,
    )
    .context("Parsing root workspace toml")?;

    let t = workspace_toml
        .as_table()
        .context("root workspace toml is not a table, WTF?")?;
    let _ = t
        .get("workspace")
        .context("Cargo.toml missing 'workspace' item. Are you running from a workspace?")?;

    let defs_directory = cwd.join("horizon-ipcdef/defs");
    let gen_directory = cwd.join("horizon-ipcdef/src/gen");

    if !defs_directory.exists() {
        return Err(anyhow!(
            "defs directory does not exist (path = {:?})",
            defs_directory
        ));
    }
    if !gen_directory.exists() {
        return Err(anyhow!(
            "gen directory does not exist (path = {:?})",
            gen_directory
        ));
    }

    Ok(Paths {
        defs_directory,
        gen_directory,
    })
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,
    pub content: String,
    /// The starting byte indices in the source code.
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: String, content: String) -> Self {
        Self {
            name,
            line_starts: codespan_reporting::files::line_starts(&content).collect(),
            content,
        }
    }

    /// Return the starting byte index of the line with the specified line index.
    /// Convenience method that already generates errors if necessary.
    fn line_start(&self, line_index: usize) -> Result<usize, codespan_reporting::files::Error> {
        use std::cmp::Ordering;

        match line_index.cmp(&self.line_starts.len()) {
            Ordering::Less => Ok(self
                .line_starts
                .get(line_index)
                .cloned()
                .expect("failed despite previous check")),
            Ordering::Equal => Ok(self.content.len()),
            Ordering::Greater => Err(codespan_reporting::files::Error::LineTooLarge {
                given: line_index,
                max: self.line_starts.len() - 1,
            }),
        }
    }
}

impl<'a> codespan_reporting::files::Files<'a> for SourceFile {
    type FileId = ();
    type Name = String;
    type Source = &'a str;

    fn name(&'a self, _: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
        Ok(self.name.clone())
    }

    fn source(&'a self, _: Self::FileId) -> Result<Self::Source, codespan_reporting::files::Error> {
        Ok(&self.content)
    }

    fn line_index(
        &'a self,
        _: Self::FileId,
        byte_index: usize,
    ) -> Result<usize, codespan_reporting::files::Error> {
        Ok(self
            .line_starts
            .binary_search(&byte_index)
            .unwrap_or_else(|next_line| next_line - 1))
    }

    fn line_range(
        &'a self,
        _: Self::FileId,
        line_index: usize,
    ) -> Result<Range<usize>, codespan_reporting::files::Error> {
        let line_start = self.line_start(line_index)?;
        let next_line_start = self.line_start(line_index + 1)?;

        Ok(line_start..next_line_start)
    }
}

#[derive(Debug, Clone)]
pub struct SourceFiles {
    pub files: Vec<SourceFile>,
}

impl SourceFiles {
    /// Add a file to the database, returning the handle that can be used to
    /// refer to it again.
    pub fn add(&mut self, name: String, content: String) -> usize {
        let file_id = self.files.len();
        self.files.push(SourceFile::new(name, content));
        file_id
    }

    /// Get the file corresponding to the given id.
    pub fn get(&self, file_id: usize) -> Result<&SourceFile, codespan_reporting::files::Error> {
        self.files
            .get(file_id)
            .ok_or(codespan_reporting::files::Error::FileMissing)
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, &SourceFile)> {
        self.files.iter().enumerate()
    }
}

impl<'a> codespan_reporting::files::Files<'a> for SourceFiles {
    type FileId = usize;
    type Name = String;
    type Source = &'a str;

    fn name(&self, file_id: usize) -> Result<Self::Name, codespan_reporting::files::Error> {
        Ok(self.get(file_id)?.name.clone())
    }

    fn source(&self, file_id: usize) -> Result<&str, codespan_reporting::files::Error> {
        Ok(self.get(file_id)?.content.as_ref())
    }

    fn line_index(
        &self,
        file_id: usize,
        byte_index: usize,
    ) -> Result<usize, codespan_reporting::files::Error> {
        self.get(file_id)?.line_index((), byte_index)
    }

    fn line_range(
        &self,
        file_id: usize,
        line_index: usize,
    ) -> Result<Range<usize>, codespan_reporting::files::Error> {
        self.get(file_id)?.line_range((), line_index)
    }
}

fn collect_source_files(defs_directory: &Path) -> anyhow::Result<SourceFiles> {
    let mut filenames = Vec::new();

    let mut files = SourceFiles { files: Vec::new() };

    for entry in walkdir::WalkDir::new(defs_directory) {
        let entry = entry.context("Walking the defs directory")?;

        if entry.file_type().is_file() {
            let path = entry.path().strip_prefix(defs_directory).unwrap();

            if path.extension() == Some(OsStr::new("id")) {
                filenames.push(
                    path.to_str()
                        .expect("Please use UTF-8 filenames")
                        .to_string(),
                );
            }
        }
    }

    for filename in filenames {
        let content = std::fs::read_to_string(defs_directory.join(&filename))
            .with_context(|| format!("Reading source file `{}`", filename))?;

        files.add(filename, content);
    }

    Ok(files)
}

/// Parse & typecheck a collection of files in a single pass as a one merged file
fn parse_files(files: &SourceFiles) -> crate::swipc::diagnostics::Result<TypecheckedIpcFile> {
    let mut res_file = IpcFile::new();

    let mut res = Ok(());

    for (id, file) in files.iter() {
        match IpcFileParser::new().parse(id, &file.content) {
            Ok(f) => res_file.merge_with(f),
            Err(e) => res.extend(diagnostics_from_parse_error(id, &file.content, e)),
        }
    }

    if let Err(e) = res {
        return Err(e);
    }

    res_file.typecheck()
}

fn display_diagnostics(files: &SourceFiles, diagnostics: crate::swipc::diagnostics::Error) {
    let mut writer =
        codespan_reporting::term::termcolor::StandardStream::stdout(ColorChoice::Always);
    let config = codespan_reporting::term::Config::default();

    for diag in diagnostics {
        codespan_reporting::term::emit(&mut writer, &config, files, &diag).unwrap();
    }
}

fn delete_dir_contents(read_dir_res: Result<ReadDir, std::io::Error>) -> anyhow::Result<()> {
    let dir = read_dir_res.context("Reading dir to delete")?;

    for entry in dir {
        let entry = entry.context("Reading dir to delete")?;
        let path = entry.path();

        if path.is_dir() {
            std::fs::remove_dir_all(path).context("Removing directory in a directory")?;
        } else {
            std::fs::remove_file(path).context("Removing file in a directory")?;
        }
    }

    Ok(())
}

fn write_files(gen_directory: &Path, files: &BTreeMap<String, String>) -> anyhow::Result<()> {
    delete_dir_contents(std::fs::read_dir(gen_directory)).context("Cleaning up gen directory")?;

    for (name, contents) in files {
        let path = gen_directory.join(name);

        std::fs::create_dir_all(path.parent().unwrap())
            .with_context(|| format!("Crating a directory for `{}`", name))?;

        std::fs::write(path, contents).with_context(|| format!("Writing `{}`", name))?;
    }

    Ok(())
}

pub fn run(args: Args) -> anyhow::Result<()> {
    match args.command {
        Command::GenIpcdef {} => {
            let paths = get_paths().context("Getting workspace paths")?;

            let source_files =
                collect_source_files(&paths.defs_directory).context("Collecting source files")?;

            let file = match parse_files(&source_files) {
                Ok(f) => f,
                Err(diags) => {
                    display_diagnostics(&source_files, diags);

                    return Err(anyhow!("Compilation failed"));
                }
            };

            let mut tok = TokenStorage::new();
            gen_ipc_file(&mut tok, file.context(), &file);

            let files = tok
                .to_file_string()
                .context("Formatting the generated source code")?;

            write_files(&paths.gen_directory, &files).context("Writing output files")?;

            Ok(())
        }
    }
}
