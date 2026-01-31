//! ASCII art banner for Arq CLI.

/// Bold ASCII art banner for Arq CLI (Arq with tall A, short rq)
pub const BANNER: &str = r#"
  █████╗
 ██╔══██╗ ██████╗  ██████╗
 ███████║ ██╔══██╗██╔═══██╗
 ██╔══██║ ██████╔╝██║▄▄ ██║
 ██║  ██║ ██╔══██╗╚██████╔╝
 ╚═╝  ╚═╝ ╚═╝  ╚═╝ ╚══▀▀═╝
"#;

/// Tagline displayed below the banner
pub const TAGLINE: &str = "Spec-first AI agent";

/// Returns the full banner with tagline for clap's before_help
pub fn banner_help() -> &'static str {
    concat!(
        "\n",
        "  █████╗                    \n",
        " ██╔══██╗ ██████╗  ██████╗  \n",
        " ███████║ ██╔══██╗██╔═══██╗ \n",
        " ██╔══██║ ██████╔╝██║▄▄ ██║ \n",
        " ██║  ██║ ██╔══██╗╚██████╔╝ \n",
        " ╚═╝  ╚═╝ ╚═╝  ╚═╝ ╚══▀▀═╝  \n",
        "\n",
        "  Spec-first AI agent\n",
    )
}

/// Print banner to stdout (for TUI startup)
pub fn print_banner() {
    println!("{}", BANNER);
    println!("  {}\n", TAGLINE);
}
