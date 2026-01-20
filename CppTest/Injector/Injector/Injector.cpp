#include <afx.h>
#include <afxwin.h>

static bool TrySerialize(CObject* obj, CArchive* ar) {
    if (!obj || !ar) {
        return false;
    }
    __try {
        obj->Serialize(*ar);
        return true;
    }
    __except (EXCEPTION_EXECUTE_HANDLER) {
        return false;
    }
}

// 存根类
class CModbusSlave : public CObject {
public:
    virtual void Serialize(CArchive& ar);
    
    // 我们需要声明这个静态函数才能调用它
    static CRuntimeClass* PASCAL GetThisClass(); 
};

// 这里的符号名必须与 DLL 导出完全一致
// 在 IDA 中确认: ?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ
// 如果链接报错，可能需要改为动态获取地址 (见下文)

extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    // =========================================================
    // 步骤 1: 动态获取 CModbusSlave::GetThisClass 函数地址
    // =========================================================
    HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
    if (!hLogic) {
        ::MessageBox(NULL, _T("dllDPLogic.dll not loaded"), _T("Error"), MB_OK);
        return;
    }

    // 这一步很关键：我们需要拿到 RuntimeClass 才能创建对象
    // 这个 mangled name 是标准的 MSVC name，你可以用 Dependency Walker 确认
    typedef CRuntimeClass* (*FnGetClass)();
    FnGetClass pfnGetClass = (FnGetClass)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");

    if (!pfnGetClass) {
        // 如果找不到导出，尝试 Plan B: 也许它是 CModbusSlave::classCModbusSlave 静态变量
        ::MessageBox(NULL, _T("Cannot find GetThisClass! Is it exported?"), _T("Error"), MB_OK);
        // 如果这里失败了，告诉我，我们需要换个方式拿 Class
        return;
    }

    // =========================================================
    // 步骤 2: 创建对象
    // =========================================================
    CRuntimeClass* pClass = pfnGetClass();
    if (!pClass) {
        ::MessageBox(NULL, _T("GetThisClass returned NULL"), _T("Error"), MB_OK);
        return;
    }

    CObject* pObj = pClass->CreateObject();
    if (!pObj) {
        ::MessageBox(NULL, _T("CreateObject failed"), _T("Error"), MB_OK);
        return;
    }

    // =========================================================
    // 步骤 3: 导出空对象
    // =========================================================
    const TCHAR* filename = _T("C:\\empty_slave.bin");
    CFile f;
    if (!f.Open(filename, CFile::modeCreate | CFile::modeWrite | CFile::typeBinary)) {
        ::MessageBox(NULL, _T("Cannot create file"), _T("Error"), MB_OK);
        return;
    }

    CArchive ar(&f, CArchive::store);
    bool bSuccess = TrySerialize(pObj, &ar);
    if (!bSuccess) {
        ::MessageBox(NULL, _T("Crash inside Serialize (Access Violation)"), _T("Dump Failed"), MB_OK | MB_ICONERROR);
    }

    ar.Close();
    f.Close();

    // 清理内存
    delete pObj;

    if (bSuccess) {
        ::MessageBox(NULL, _T("✅ EMPTY DUMP SUCCESS!\nFile: C:\\empty_slave.bin"), _T("Victory"), MB_OK);
    }
}
