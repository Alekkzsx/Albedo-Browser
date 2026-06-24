use fxhash::FxHashMap;
use memchr::memchr;
use phf::{phf_map, phf_set};
use smol_str::SmolStr;
use super::types::{HtmlDocument, HtmlNode, ParserOptions, PreloadRequest, ResourceType, RequestPriority};


pub mod extract_preloads; pub use extract_preloads::*;
pub mod collect_preloads; pub use collect_preloads::*;
pub mod absolutize_url; pub use absolutize_url::*;
pub mod preloadtagkind; pub use preloadtagkind::*;
pub mod preload_tags; pub use preload_tags::*;
pub mod preload_attrs; pub use preload_attrs::*;
pub mod collect_preloads_fast; pub use collect_preloads_fast::*;
pub mod might_have_preload_tags; pub use might_have_preload_tags::*;
pub mod ascii_tag_starts_with; pub use ascii_tag_starts_with::*;
pub mod scan_preloads_fast; pub use scan_preloads_fast::*;
pub mod find_tag_end; pub use find_tag_end::*;
pub mod collect_relevant_attrs; pub use collect_relevant_attrs::*;
pub mod preload_from_link_attrs; pub use preload_from_link_attrs::*;
pub mod preload_from_script_attrs; pub use preload_from_script_attrs::*;
pub mod ascii_lower_smol; pub use ascii_lower_smol::*;
pub mod bytes_to_smol; pub use bytes_to_smol::*;
