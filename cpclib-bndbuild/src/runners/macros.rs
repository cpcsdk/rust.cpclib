/// Internal macro to generate the command builder with help/version flags
/// 
/// This is used internally by other macros to avoid code duplication.
#[doc(hidden)]
#[macro_export]
macro_rules! __define_runner_command_builder {
    // Variant for clap derive (uses CommandFactory)
    (derive: $args_type:ty, $after_help:expr) => {
        <$args_type as clap::CommandFactory>::command()
            .no_binary_name(true)
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                clap::Arg::new("help")
                    .long("help")
                    .short('h')
                    .action(clap::ArgAction::SetTrue)
                    .exclusive(true)
            )
            .arg(
                clap::Arg::new("version")
                    .long("version")
                    .short('V')
                    .help("Print version")
                    .action(clap::ArgAction::SetTrue)
                    .exclusive(true)
            )
            .after_help($after_help)
    };
    // Variant for custom builder function
    (builder: $builder_fn:expr, $after_help:expr) => {
        {
            let command = $builder_fn;
            command
                .no_binary_name(true)
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    clap::Arg::new("help")
                        .long("help")
                        .short('h')
                        .action(clap::ArgAction::SetTrue)
                        .exclusive(true)
                )
                .arg(
                    clap::Arg::new("version")
                        .long("version")
                        .short('V')
                        .help("Print version")
                        .action(clap::ArgAction::SetTrue)
                        .exclusive(true)
                )
                .after_help($after_help)
        }
    };
}

/// Internal macro to generate struct and basic impls
/// 
/// This is used internally by other macros to avoid code duplication.
#[doc(hidden)]
#[macro_export]
macro_rules! __define_runner_struct_and_impls {
    ($runner_name:ident, $command_expr:expr) => {
        pub struct $runner_name<E: EventObserver> {
            command: clap::Command,
            _phantom: std::marker::PhantomData<E>
        }

        impl<E: EventObserver> Default for $runner_name<E> {
            fn default() -> Self {
                Self {
                    command: $command_expr,
                    _phantom: Default::default()
                }
            }
        }

        impl<E: EventObserver> cpclib_runner::runner::RunnerWithClap for $runner_name<E> {
            fn get_clap_command(&self) -> &clap::Command {
                &self.command
            }
        }
    };
}

/// Internal macro to generate the Runner impl
/// 
/// This is used internally by other macros to avoid code duplication.
#[doc(hidden)]
#[macro_export]
macro_rules! __define_runner_impl {
    // Variant for clap derive runners (uses get_args)
    (derive: $runner_name:ident, $command_name:expr, $process_fn:expr) => {
        impl<E: EventObserver> cpclib_runner::runner::Runner for $runner_name<E> {
            type EventObserver = E;

            fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
                let Some(cli) = self.get_args(itr, o)? else {
                    return Ok(());
                };
                $process_fn(cli)
            }

            fn get_command(&self) -> &str {
                $command_name
            }
        }
    };
    // Variant for custom builder runners with command reference
    (matches_with_cmd: $runner_name:ident, $command_name:expr, $process_fn:expr) => {
        impl<E: EventObserver> cpclib_runner::runner::Runner for $runner_name<E> {
            type EventObserver = E;

            fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
                let Some(matches) = self.get_matches(itr, o)? else {
                    return Ok(());
                };
                $process_fn(matches, &self.command)
            }

            fn get_command(&self) -> &str {
                $command_name
            }
        }
    };
    // Variant for custom builder runners without command reference
    (matches: $runner_name:ident, $command_name:expr, $process_fn:expr) => {
        impl<E: EventObserver> cpclib_runner::runner::Runner for $runner_name<E> {
            type EventObserver = E;

            fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
                let Some(matches) = self.get_matches(itr, o)? else {
                    return Ok(());
                };
                $process_fn(matches)
            }

            fn get_command(&self) -> &str {
                $command_name
            }
        }
    };
}

/// Macro to define a runner based on clap_derive Args
/// 
/// This macro generates the boilerplate for runners that use `#[derive(Parser)]` Args.
/// 
/// # Example
/// ```ignore
/// define_clap_derive_runner! {
///     CslRunner,                           // Runner struct name
///     CslCliArgs,                          // Args type (must impl CommandFactory)
///     CSL_CMDS[0],                         // Command name
///     env!("CARGO_PKG_VERSION"),           // Package version
///     |cli| cpclib_cslcli::run(&cli).map_err(|e| e.to_string())  // Processing function
/// }
/// ```
#[macro_export]
macro_rules! define_clap_derive_runner {
    (
        $runner_name:ident,
        $args_type:ty,
        $command_name:expr,
        $pkg_version:expr,
        $process_fn:expr
    ) => {
        $crate::__define_runner_struct_and_impls!(
            $runner_name,
            $crate::__define_runner_command_builder!(
                derive: $args_type,
                format!(
                    "{} embedded by {} {}",
                    $pkg_version,
                    $crate::built_info::PKG_NAME,
                    $crate::built_info::PKG_VERSION
                )
            ).bin_name($command_name)
        );

        impl<E: EventObserver> cpclib_runner::runner::runner::RunnerWithClapDerive for $runner_name<E> {
            type Args = $args_type;
        }

        $crate::__define_runner_impl!(derive: $runner_name, $command_name, $process_fn);
    };
}

/// Macro to define a runner based on a custom clap Command builder function
/// 
/// This macro generates the boilerplate for runners that use a function to build
/// a clap::Command (like `build_args_parser()`).
/// 
/// Supports two variants:
/// - With command reference: process function takes (matches, command)
/// - Without command reference: process function takes (matches) only
/// 
/// # Examples
/// ```ignore
/// // With command reference (2-arg closure)
/// define_custom_builder_runner! {
///     BasmDocRunner,
///     cpclib_basmdoc::cmdline::build_args_parser(),
///     BASMDOC_CMD,
///     cpclib_basmdoc::built_info::PKG_NAME,
///     cpclib_basmdoc::built_info::PKG_VERSION,
///     |matches, command| cpclib_basmdoc::cmdline::handle_matches(&matches, command).map_err(|e| e.to_string())
/// }
/// 
/// // Without command reference (1-arg closure) - use 'simple' variant
/// define_custom_builder_runner! {
///     simple:
///     SomeRunner,
///     build_parser(),
///     "command",
///     "pkg",
///     "1.0",
///     |matches| process(matches)
/// }
/// ```
#[macro_export]
macro_rules! define_custom_builder_runner {
    // Variant without command reference (simple)
    (
        simple:
        $runner_name:ident,
        $builder_fn:expr,
        $command_name:expr,
        $pkg_name:expr,
        $pkg_version:expr,
        $process_fn:expr
    ) => {
        $crate::__define_runner_struct_and_impls!(
            $runner_name,
            $crate::__define_runner_command_builder!(
                builder: $builder_fn,
                format!(
                    "{} {} embedded by {} {}",
                    $pkg_name,
                    $pkg_version,
                    $crate::built_info::PKG_NAME,
                    $crate::built_info::PKG_VERSION
                )
            )
        );

        impl<E: EventObserver> cpclib_runner::runner::runner::RunnerWithClapMatches for $runner_name<E> {}

        $crate::__define_runner_impl!(matches: $runner_name, $command_name, $process_fn);
    };
    // Variant with command reference (default)
    (
        $runner_name:ident,
        $builder_fn:expr,
        $command_name:expr,
        $pkg_name:expr,
        $pkg_version:expr,
        $process_fn:expr
    ) => {
        $crate::__define_runner_struct_and_impls!(
            $runner_name,
            $crate::__define_runner_command_builder!(
                builder: $builder_fn,
                format!(
                    "{} {} embedded by {} {}",
                    $pkg_name,
                    $pkg_version,
                    $crate::built_info::PKG_NAME,
                    $crate::built_info::PKG_VERSION
                )
            )
        );

        impl<E: EventObserver> cpclib_runner::runner::runner::RunnerWithClapMatches for $runner_name<E> {}

        $crate::__define_runner_impl!(matches_with_cmd: $runner_name, $command_name, $process_fn);
    };
}

/// Simplified variant when the process function doesn't need the command reference
/// 
/// **Deprecated**: Use `define_custom_builder_runner!` with the `simple:` prefix instead.
/// This macro is kept for backward compatibility.
#[macro_export]
#[deprecated(since = "0.11.0", note = "Use `define_custom_builder_runner! { simple: ... }` instead")]
macro_rules! define_custom_builder_runner_simple {
    (
        $runner_name:ident,
        $builder_fn:expr,
        $command_name:expr,
        $pkg_name:expr,
        $pkg_version:expr,
        $process_fn:expr
    ) => {
        $crate::define_custom_builder_runner!(
            simple:
            $runner_name,
            $builder_fn,
            $command_name,
            $pkg_name,
            $pkg_version,
            $process_fn
        );
    };
}

/// Macro to define the boilerplate for file system runners using clap derive
/// 
/// This macro generates the struct and standard impls, but leaves `inner_run` 
/// and `get_command` for manual implementation since file system operations 
/// have custom logic that varies per runner.
/// 
/// # Example
/// ```ignore
/// define_fs_runner_struct! {
///     CpRunner,     // Runner struct name
///     CpArgs        // Args type (must impl CommandFactory)
/// }
/// 
/// // Then manually implement Runner with custom inner_run logic
/// impl<E: EventObserver> Runner for CpRunner<E> {
///     type EventObserver = E;
///     
///     fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
///         // Custom file system logic here
///         let Some(matches) = self.get_matches(itr, o)? else {
///             return Ok(());
///         };
///         let args = CpArgs::from_arg_matches(&matches)
///             .map_err(|e| e.to_string())?;
///         // ... rest of custom logic
///     }
///     
///     fn get_command(&self) -> &str {
///         CP_CMDS[0]
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_fs_runner_struct {
    ($runner_name:ident, $args_type:ty) => {
        $crate::__define_runner_struct_and_impls!(
            $runner_name,
            $crate::__define_runner_command_builder!(
                derive: $args_type,
                format!(
                    "Inner command of {} {}",
                    $crate::built_info::PKG_NAME,
                    $crate::built_info::PKG_VERSION
                )
            )
        );
    };
}
