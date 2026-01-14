use std::ffi::CString;
use std::ptr;
use anyhow::{bail, Context, Result};

// 引入 Windows API 类型
use windows::core::PCSTR;
use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatA, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

/// 将二进制数据写入系统剪贴板
/// format_name: 必须与 PLC 软件识别的格式完全一致
pub fn write_to_clipboard(format_name: &str, data: &[u8]) -> Result<()> {
    // 1. 将 Rust 字符串转换为 C 风格字符串 (以 \0 结尾)
    let format_cstr = CString::new(format_name)
        .context("无法将目标字符串转换为C风格字符串")?;

    unsafe {
        // 2. 打开剪贴板 (HWND(0) 表示当前窗口)
        if !OpenClipboard(HWND(0)).as_bool() {
            bail!("打开粘贴板失败");
        }

        // RAII 守卫：确保函数退出时自动关闭剪贴板，防止死锁其他程序
        struct ClipboardGuard;
        impl Drop for ClipboardGuard {
            fn drop(&mut self) {
                unsafe { let _ = CloseClipboard(); }
            }
        }
        let _guard = ClipboardGuard;

        // 3. 清空剪贴板
        if !EmptyClipboard().as_bool() {
            bail!("无法清空粘贴板");
        }

        // 4. 注册自定义格式 (关键步骤)
        // 如果格式已存在，Windows 返回现有 ID；否则分配新 ID
        let format_id = RegisterClipboardFormatA(PCSTR(format_cstr.as_ptr() as *const u8));
        if format_id == 0 {
            bail!("注册自定义粘贴板失败: {}", format_name);
        }

        // 5. 分配全局内存 (GlobalAlloc)
        // Windows 剪贴板要求数据必须放在全局堆中，且是 MOVEABLE 的
        let hglobal = GlobalAlloc(GMEM_MOVEABLE, data.len()).context("内存分配失败")?;

        // 6. 锁定内存并写入数据
        // GlobalLock 返回内存块的第一个字节指针
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            let _ = GlobalFree(hglobal);
            bail!("锁定内存失败");
        }

        // 内存拷贝：将 Rust 的 Vec 数据复制到 Windows 全局内存中
        ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());

        // 解锁内存
        let _ = GlobalUnlock(hglobal);

        // --- 修正重点 3: HGLOBAL 转换与所有权移交 ---
        // SetClipboardData 需要 HANDLE 类型，而 hglobal 是 HGLOBAL 类型。
        // 在 windows crate 中，它们是不同的结构体，但内部都包含一个 *mut c_void (即 .0)
        // 我们需要手动构造 HANDLE。
        let handle = HANDLE(hglobal.0);
        
        // --- 修正重点 4: SetClipboardData 返回 Result ---
        // 如果成功，SetClipboardData 返回 Ok(HANDLE)；如果失败，返回 Err。
        // 关键逻辑：
        // 1. 如果成功：系统接管内存所有权，我们不能再 Free 它。
        // 2. 如果失败：系统不接管，我们必须手动 GlobalFree，否则内存泄漏。
        if let Err(e) = SetClipboardData(format_id, handle) {
            let _ = GlobalFree(hglobal); // 关键：失败时回收内存
            return Err(e).context("设定粘贴板数据失败");
            // 成功时，hglobal 的所有权已移交给剪贴板，函数结束时不释放它。
        }
    }

    Ok(())
}