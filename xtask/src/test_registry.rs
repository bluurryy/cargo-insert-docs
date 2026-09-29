use color_eyre::eyre::Result;

#[derive(Default)]
pub struct TestRegistry {
    tests: Vec<(&'static str, fn() -> Result)>,
}

impl TestRegistry {
    pub fn add(&mut self, name: &'static str, f: fn() -> Result) {
        self.tests.push((name, f))
    }

    pub fn run(&self, filter: Option<&str>) {
        for (name, f) in &self.tests {
            if let Some(filter) = filter
                && !name.contains(filter)
            {
                continue;
            }

            let text_style = anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White)))
                .bold()
                .italic();

            let name_style = anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)))
                .bold()
                .italic();

            let text = "STARTING TEST";
            let lines = "─".repeat(text.len() + 1 + name.len());

            eprintln!();
            eprintln!("╭─{lines}─╮");
            eprintln!("│ {text_style}{text}{text_style:#} {name_style}{name}{name_style:#} │");
            eprintln!("╰─{lines}─╯");
            eprintln!();

            f().expect(&format!("test \"{name}\" failed"));
        }
    }
}
