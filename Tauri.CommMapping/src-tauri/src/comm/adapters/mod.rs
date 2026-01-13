//! 适配器层（adapters）：第三方库/IO/平台相关实现（driver、storage、xlsx parser 等）。

pub mod driver;
pub mod storage;
pub mod union_xlsx_parser;
