#include <afx.h>
#include <afxwin.h>

extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    void* pRealObj = (void*)0x0F2D2A04;

    if (IsBadReadPtr(pRealObj, 4)) {
        ::MessageBox(NULL, "Address invalid! Please rescan.", "Error", MB_OK);
        return;
    }

    CFile f;
    if (f.Open("C:\\valid_dump.bin", CFile::modeCreate | CFile::modeWrite | CFile::typeBinary)) {
        CArchive ar(&f, CArchive::store);
        try {
            ((CObject*)pRealObj)->Serialize(ar);
            ar.Close();
            f.Close();
            ::MessageBox(NULL, "Dump Success! Check C:\\valid_dump.bin", "Success", MB_OK);
        } catch (...) {
            ::MessageBox(NULL, "Dump Failed (Exception)", "Error", MB_OK);
        }
    }
}
