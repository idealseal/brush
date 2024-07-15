use crate::{builtin, commands, error};
use cfg_if::cfg_if;
use clap::Parser;
#[cfg(not(target_os = "linux"))]
use nix::sys::stat::Mode;
use std::io::Write;

/// Manage the process umask.
#[derive(Parser)]
pub(crate) struct UmaskCommand {
    /// If MODE is omitted, output in a form that may be reused as input.
    #[arg(short = 'p')]
    print_roundtrippable: bool,

    /// Makes the output symbolic; otherwise an octal number is given.
    #[arg(short = 'S')]
    symbolic_output: bool,

    /// Mode mask.
    mode: Option<String>,
}

#[async_trait::async_trait]
impl builtin::Command for UmaskCommand {
    async fn execute(
        &self,
        context: commands::ExecutionContext<'_>,
    ) -> Result<crate::builtin::ExitCode, crate::error::Error> {
        if let Some(mode) = &self.mode {
            if mode.starts_with(|c: char| c.is_digit(8)) {
                let parsed = u32::from_str_radix(mode.as_str(), 8)?;
                set_umask(parsed)?;
            } else {
                return crate::error::unimp("umask setting mode from symbolic value");
            }
        } else {
            let umask = get_umask()?;

            let formatted = if self.symbolic_output {
                let u = symbolic_mask_from_bits((!umask & 0o700) >> 6);
                let g = symbolic_mask_from_bits((!umask & 0o070) >> 3);
                let o = symbolic_mask_from_bits(!umask & 0o007);
                std::format!("u={u},g={g},o={o}")
            } else {
                std::format!("{umask:04o}")
            };

            if self.print_roundtrippable {
                writeln!(context.stdout(), "umask {formatted}")?;
            } else {
                writeln!(context.stdout(), "{formatted}")?;
            }
        }

        Ok(builtin::ExitCode::Success)
    }
}

cfg_if! {
    if #[cfg(target_os = "linux")] {
        fn get_umask() -> Result<u32, error::Error> {
            let me = procfs::process::Process::myself()?;
            let status = me.status()?;
            status.umask.ok_or_else(|| error::Error::InvalidUmask)
        }
    } else {
        fn get_umask() -> Result<u32, error::Error> {
            let u = nix::sys::stat::umask(Mode::empty());
            nix::sys::stat::umask(u);
            Ok(u.bits() as u32)
        }
    }
}

fn set_umask(value: u32) -> Result<(), error::Error> {
    let mode =
        nix::sys::stat::Mode::from_bits(value as _).ok_or_else(|| error::Error::InvalidUmask)?;
    nix::sys::stat::umask(mode);
    Ok(())
}

fn symbolic_mask_from_bits(bits: u32) -> String {
    let mut result = String::new();

    if (bits & 0b100) != 0 {
        result.push('r');
    }
    if (bits & 0b010) != 0 {
        result.push('w');
    }
    if (bits & 0b001) != 0 {
        result.push('x');
    }

    result
}
