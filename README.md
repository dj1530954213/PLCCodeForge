# PLCCodeForge

PLC程序自动编程工具 - 基于 Tauri + Vue 3 + TypeScript 的桌面应用

## 简介

PLCCodeForge 是一个用于自动生成 PLC（可编程逻辑控制器）程序的桌面工具，旨在提高工业自动化开发效率，减少重复性编码工作。

## 技术栈

- **前端**: Vue 3 + TypeScript + Vite
- **后端**: Rust + Tauri 2.0
- **UI**: (待选型)

## 功能特性

- **自动代码生成** - 根据配置自动生成 ST/IL/LD 等 IEC 61131-3 标准代码
- **模板化编程** - 支持自定义代码模板，快速复用
- **多品牌支持** - 适配西门子、三菱、欧姆龙等主流 PLC 品牌
- **点表导入** - 支持从 Excel/CSV 导入 IO 点表自动生成变量声明
- **语法检查** - 内置代码规范检查与验证
- **跨平台** - 支持 Windows、macOS、Linux

## 快速开始

```bash
# 安装依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建生产版本
npm run tauri build
```

## 项目结构

```
PLCCodeForge/
├── src/                # Vue前端源码
├── src-tauri/          # Rust后端源码
│   ├── src/            # Rust源代码
│   └── Cargo.toml      # Rust依赖配置
├── public/             # 静态资源
├── package.json        # Node.js依赖配置
└── README.md
```

## 开发环境要求

- Node.js >= 18
- Rust >= 1.70
- 参考 [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)

## 许可证

MIT License
