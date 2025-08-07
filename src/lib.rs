// These links overwrite the ones in `README.md`
// to become proper intra-doc links in Rust docs.
//! [`.freeze()`]: FreezableRecorder::freeze()
//! [`Arc`]: std::sync::Arc
//! [`arc-swap`]: arc_swap
//! [`AtomicBool`]: std::sync::atomic::AtomicBool
//! [`Describable`]: metric::Describable
//! [`failure::strategy`]: failure::strategy
//! [`failure::Strategy`]: failure::Strategy
//! [`FreezableRecorder`]: FreezableRecorder
//! [`FrozenRecorder`]: FrozenRecorder
//! [`HashMap`]: std::collections::HashMap
//! [`metrics`]: metrics
//! [`metrics::Counter`]: metrics::Counter
//! [`metrics::Counter::noop()`]: metrics::Counter::noop()
//! [`metrics::Gauge`]: metrics::Gauge
//! [`metrics::Histogram`]: metrics::Histogram
//! [`metrics::Recorder`]: metrics::Recorder
//! [`metrics::Registry`]: metrics_util::registry::Registry
//! [`metrics::Unit`]: metrics::Unit
//! [`PanicInDebugNoOpInRelease`]: failure::strategy::PanicInDebugNoOpInRelease
//! [`prometheus`]: prometheus
//! [`prometheus::Error`]: prometheus::Error
//! [`prometheus::Gauge`]: prometheus::Gauge
//! [`prometheus::GaugeVec`]: prometheus::GaugeVec
//! [`prometheus::Histogram`]: prometheus::Histogram
//! [`prometheus::HistogramVec`]: prometheus::HistogramVec
//! [`prometheus::IntCounter`]: prometheus::IntCounter
//! [`prometheus::IntCounterVec`]: prometheus::IntCounterVec
//! [`prometheus::MetricVec`]: prometheus::core::MetricVec
//! [`prometheus::Registry`]: prometheus::Registry
//! [`read`-lock]: std::sync::RwLock::read()
//! [`Recorder`]: Recorder
#![cfg_attr(any(doc, test), doc = include_str!("../README.md"))]
#![cfg_attr(not(any(doc, test)), doc = env!("CARGO_PKG_NAME"))]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/instrumentisto\
                     /metrics-prometheus-rs\
                     /80bcffc2096f9ff213ec84833a9d8dd81a115cd5/logo.png",
    html_favicon_url = "https://raw.githubusercontent.com/instrumentisto\
                        /metrics-prometheus-rs\
                        /80bcffc2096f9ff213ec84833a9d8dd81a115cd5/logo.png"
)]
#![deny(nonstandard_style, rustdoc::all, trivial_casts, trivial_numeric_casts)]
#![forbid(non_ascii_idents, unsafe_code)]
#![warn(
    clippy::absolute_paths,
    clippy::allow_attributes,
    clippy::allow_attributes_without_reason,
    clippy::as_conversions,
    clippy::as_pointer_underscore,
    clippy::as_ptr_cast_mut,
    clippy::assertions_on_result_states,
    clippy::branches_sharing_code,
    clippy::cfg_not_test,
    clippy::clear_with_drain,
    clippy::clone_on_ref_ptr,
    clippy::coerce_container_to_any,
    clippy::collection_is_never_read,
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::decimal_literal_representation,
    clippy::default_union_representation,
    clippy::derive_partial_eq_without_eq,
    clippy::doc_include_without_cfg,
    clippy::empty_drop,
    clippy::empty_structs_with_brackets,
    clippy::equatable_if_let,
    clippy::empty_enum_variants_with_brackets,
    clippy::exit,
    clippy::expect_used,
    clippy::fallible_impl_from,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast_any,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::imprecise_flops,
    clippy::infinite_loop,
    clippy::iter_on_empty_collections,
    clippy::iter_on_single_items,
    clippy::iter_over_hash_type,
    clippy::iter_with_drain,
    clippy::large_include_file,
    clippy::large_stack_frames,
    clippy::let_underscore_untyped,
    clippy::literal_string_with_formatting_args,
    clippy::lossy_float_literal,
    clippy::map_err_ignore,
    clippy::map_with_unused_argument_over_ranges,
    clippy::mem_forget,
    clippy::missing_assert_message,
    clippy::missing_asserts_for_indexing,
    clippy::missing_const_for_fn,
    clippy::missing_docs_in_private_items,
    clippy::module_name_repetitions,
    clippy::multiple_inherent_impl,
    clippy::multiple_unsafe_ops_per_block,
    clippy::mutex_atomic,
    clippy::mutex_integer,
    clippy::needless_collect,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_raw_strings,
    clippy::non_zero_suggestions,
    clippy::nonstandard_macro_braces,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic_in_result_fn,
    clippy::partial_pub_fields,
    clippy::pathbuf_init_then_push,
    clippy::pedantic,
    clippy::precedence_bits,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_without_shorthand,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::read_zero_byte_vec,
    clippy::redundant_clone,
    clippy::redundant_test_prefix,
    clippy::redundant_type_annotations,
    clippy::renamed_function_params,
    clippy::ref_patterns,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::return_and_then,
    clippy::same_name_method,
    clippy::semicolon_inside_block,
    clippy::set_contains_or_insert,
    clippy::shadow_unrelated,
    clippy::significant_drop_in_scrutinee,
    clippy::significant_drop_tightening,
    clippy::single_option_map,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_lit_chars_any,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::suboptimal_flops,
    clippy::suspicious_operation_groupings,
    clippy::suspicious_xor_used_as_pow,
    clippy::tests_outside_test_module,
    clippy::todo,
    clippy::too_long_first_doc_paragraph,
    clippy::trailing_empty_array,
    clippy::transmute_undefined_repr,
    clippy::trivial_regex,
    clippy::try_err,
    clippy::undocumented_unsafe_blocks,
    clippy::unimplemented,
    clippy::uninhabited_references,
    clippy::unnecessary_safety_comment,
    clippy::unnecessary_safety_doc,
    clippy::unnecessary_self_imports,
    clippy::unnecessary_struct_initialization,
    clippy::unused_peekable,
    clippy::unused_result_ok,
    clippy::unused_trait_names,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug,
    clippy::use_self,
    clippy::useless_let_if_seq,
    clippy::verbose_file_reads,
    clippy::while_float,
    clippy::wildcard_enum_match_arm,
    ambiguous_negative_literals,
    closure_returning_async_block,
    future_incompatible,
    impl_trait_redundant_captures,
    let_underscore_drop,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    redundant_lifetimes,
    rust_2018_idioms,
    single_use_lifetimes,
    unit_bindings,
    unnameable_types,
    unreachable_pub,
    unstable_features,
    unused,
    variant_size_differences
)]

pub mod failure;
pub mod metric;
pub mod recorder;
pub mod storage;

#[doc(inline)]
pub use self::{
    metric::Metric,
    recorder::{
        Freezable as FreezableRecorder, Frozen as FrozenRecorder, Recorder,
    },
};

/// Tries to install a default [`Recorder`] (backed by the
/// [`prometheus::default_registry()`]) with the
/// [`metrics::set_global_recorder()`].
///
/// # Errors
///
/// If the [`Recorder`] fails to be installed with the
/// [`metrics::set_global_recorder()`].
pub fn try_install() -> Result<Recorder, metrics::SetRecorderError<Recorder>> {
    Recorder::builder().try_build_and_install()
}

/// Tries to install a default [`FreezableRecorder`] (backed by the
/// [`prometheus::default_registry()`]) with the
/// [`metrics::set_global_recorder()`].
///
/// # Errors
///
/// If the [`FreezableRecorder`] fails to be installed with the
/// [`metrics::set_global_recorder()`].
pub fn try_install_freezable()
-> Result<FreezableRecorder, metrics::SetRecorderError<FreezableRecorder>> {
    Recorder::builder().try_build_freezable_and_install()
}

/// Installs a default [`Recorder`] (backed by the
/// [`prometheus::default_registry()`]) with the
/// [`metrics::set_global_recorder()`].
///
/// # Panics
///
/// If the [`Recorder`] fails to be installed with the
/// [`metrics::set_global_recorder()`].
#[expect( // intentional
    clippy::must_use_candidate,
    reason = "`#[must_use]` is omitted here, to avoid forcing library users \
              using the returned `Recorder` directly"
)]
pub fn install() -> Recorder {
    Recorder::builder().build_and_install()
}

/// Installs a default [`FreezableRecorder`] (backed by the
/// [`prometheus::default_registry()`]) with the
/// [`metrics::set_global_recorder()`].
///
/// # Panics
///
/// If the [`FreezableRecorder`] fails to be installed with the
/// [`metrics::set_global_recorder()`].
#[expect( // intentional
    clippy::must_use_candidate,
    reason = "`#[must_use]` is omitted here, to avoid forcing library users \
              using the returned `Recorder` directly"
)]
pub fn install_freezable() -> FreezableRecorder {
    Recorder::builder().build_freezable_and_install()
}
