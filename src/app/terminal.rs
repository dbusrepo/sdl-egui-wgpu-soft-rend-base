use std::io::{self, Write};

#[allow(clippy::print_stdout)]
pub(super) fn clear_terminal() -> io::Result<()> {
    // ANSI escape code to clear screen and move the cursor to the top-left corner.
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush()
}
