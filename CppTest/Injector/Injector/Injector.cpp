#include <afx.h>
#include <afxwin.h>
#include <vector>
#include <afxtempl.h>

typedef CRuntimeClass* (__stdcall* PGET_CLASS)();

void ShowError(LPCTSTR msg) {
    ::MessageBox(NULL, msg, _T("Injector"), MB_OK | MB_ICONERROR);
}

extern "C" __declspec(dllexport) void RunPoc()
{
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    void* pManagerRaw = *(void**)0x0084713C;
    if (!pManagerRaw) {
        ShowError(_T("Manager is NULL"));
        return;
    }

    CObject* pManager = reinterpret_cast<CObject*>(pManagerRaw);
    CString info;
    info.Format(_T("Manager Addr: %p\n"), pManager);

    try {
        CRuntimeClass* pClass = pManager->GetRuntimeClass();
        if (pClass && pClass->m_lpszClassName) {
            info.AppendFormat(_T("Class Name: %S\n"), pClass->m_lpszClassName);
        } else {
            info.Append(_T("Class Name: <null>\n"));
        }

        if (pManager->IsKindOf(RUNTIME_CLASS(CObList))) {
            info.Append(_T("✅ YES! It IS a CObList!\nWe can use standard AddTail."));
        } else {
            info.Append(_T("❌ NO. It is NOT a CObList.\nWe need to find its base class."));
        }

        ::MessageBox(NULL, info, _T("RTTI Analysis"), MB_OK);
    } catch (...) {
        ShowError(_T("Crash during RTTI check."));
    }
}
