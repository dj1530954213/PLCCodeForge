// #include <afx.h>
// #include <afxwin.h>

// class CModbusSlave : public CObject {
// public:
//     virtual void Serialize(CArchive& ar);
//     static CRuntimeClass* PASCAL GetThisClass();
// };

// extern "C" __declspec(dllexport) void RunPoc() {
//     AFX_MANAGE_STATE(AfxGetStaticModuleState());

//     // 1. 读取 Payload
//     CFile fLoad;
//     CFileException e;
//     if (!fLoad.Open(_T("C:\\payload.bin"), CFile::modeRead | CFile::typeBinary, &e)) {
//         ::MessageBox(NULL, _T("Payload not found"), 0, 0);
//         return;
//     }

//     HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
//     if (!hLogic) return;
//     typedef CRuntimeClass* (*FnGetClass)();
//     FnGetClass pfnGetClass = (FnGetClass)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
//     if (!pfnGetClass) return;

//     CObject* pObj = pfnGetClass()->CreateObject();

//     // 2. Load (反序列化)
//     CArchive arLoad(&fLoad, CArchive::load);
//     try {
//         pObj->Serialize(arLoad);
//     }
//     catch(...) {
//         ::MessageBox(NULL, _T("Load Failed!"), _T("Error"), MB_OK);
//         delete pObj;
//         return;
//     }
//     arLoad.Close();
//     fLoad.Close();

//     // 3. Round-Trip Store (再次序列化到新文件)
//     CFile fStore;
//     if (fStore.Open(_T("C:\\roundtrip.bin"), CFile::modeCreate | CFile::modeWrite | CFile::typeBinary)) {
//         CArchive arStore(&fStore, CArchive::store);
//         try {
//             pObj->Serialize(arStore);
//             ::MessageBox(NULL, _T("Round-Trip Success!\nCheck C:\\roundtrip.bin"), _T("Victory"), MB_OK);
//         }
//         catch(...) {
//             ::MessageBox(NULL, _T("Round-Trip Store Failed"), _T("Error"), MB_OK);
//         }
//         arStore.Close();
//         fStore.Close();
//     }

//     delete pObj;
// }

#include <afx.h>
#include <afxwin.h>

// =============================================================
// 1. 定义函数指针类型
// =============================================================
// GetNewID: 分配ID并写入对象
typedef void (__thiscall *FnGetNewID)(void* pContainer, CObject* pSlave);

// Register (sub_1001CF00): 存入哈希表
// 返回值是 void** (指向槽位的指针)，我们需要往里面写数据
typedef void** (__thiscall *FnRegister)(void* pContainer, int id);

// Notify (sub_1004A380): 通知UI刷新
typedef void (__thiscall *FnNotify)(void* pContainer, int* pID);

// 存根类，用于欺骗编译器调用虚函数
class CModbusSlave : public CObject {
public:
    virtual void Serialize(CArchive& ar);
    static CRuntimeClass* PASCAL GetThisClass();
};

extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    // =========================================================
    // 🔴 填入你在 CE 里搜到的地址 (截图中的地址)
    // =========================================================
    void* pContainer = (void*)0x124C23E8; 

    // 安全检查：防止地址变动导致崩溃
    if (IsBadReadPtr(pContainer, 4)) {
        ::MessageBox(NULL, _T("Container Address Invalid! Address changed?"), _T("Stop"), MB_OK);
        return;
    }

    // 1. 计算函数地址 (基址 + 偏移)
    HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
    if (!hLogic) { ::MessageBox(NULL, _T("DLL not loaded"), 0, 0); return; }
    DWORD_PTR base = (DWORD_PTR)hLogic;
    
    // ⚠️ 偏移量确认 (基于你之前的 IDA 截图)
    // GetNewID: 100471A0 -> 0x471A0
    // Register: 1001CF00 -> 0x1CF00
    // Notify:   1004A380 -> 0x4A380
    FnGetNewID GetNewID = (FnGetNewID)(base + 0x471A0);
    FnRegister Register = (FnRegister)(base + 0x1CF00);
    FnNotify   Notify   = (FnNotify)(base + 0x4A380);

    // 2. Load Payload (制造零件)
    CFile f;
    if (!f.Open(_T("C:\\payload.bin"), CFile::modeRead | CFile::typeBinary)) {
        ::MessageBox(NULL, _T("C:\\payload.bin not found!"), _T("Error"), MB_OK);
        return;
    }
    
    // 读取文件到内存
    ULONGLONG len = f.GetLength();
    BYTE* buf = new BYTE[(size_t)len];
    f.Read(buf, (UINT)len);
    f.Close();
    CMemFile mem(buf, (UINT)len);
    CArchive ar(&mem, CArchive::load);

    // 创建对象
    typedef CRuntimeClass* (*FnGetClass)();
    FnGetClass pfnGetClass = (FnGetClass)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!pfnGetClass) { ::MessageBox(NULL, _T("No GetThisClass"), 0, 0); return; }
    
    CObject* pSlave = pfnGetClass()->CreateObject();
    
    try {
        pSlave->Serialize(ar); // 反序列化
    } catch(...) {
        ::MessageBox(NULL, _T("Load Failed! Payload structure wrong?"), _T("Error"), MB_OK);
        delete pSlave; delete[] buf; return;
    }
    ar.Close(); delete[] buf;

    // =========================================================
    // 3. 执行挂载 (核心操作)
    // =========================================================
    try {
        // A. 分配 ID
        // 这会自动在 pSlave 内部填入一个新的 ID
        GetNewID(pContainer, pSlave);

        // B. 读取分配到的 ID
        // 根据 GPT 分析，ID 位于对象偏移 +24 (0x18) 处
        int id = *((int*)((char*)pSlave + 24));

        CString msg;
        msg.Format(_T("ID Allocated: %d. Injecting..."), id);
        // ::MessageBox(NULL, msg, _T("Debug"), MB_OK);

        // C. 存入哈希表 (Map[id] = pSlave)
        // 这是让数据层接纳它的关键
        void** pSlot = Register(pContainer, id);
        if (pSlot) {
            *pSlot = pSlave; 
        } else {
            ::MessageBox(NULL, _T("Register returned NULL!"), _T("Error"), MB_OK);
            return; // 不要 delete，防止二次释放
        }

        // D. 推入通知队列 (Queue.Push(id))
        // 这是让 UI 刷新显示的关键
        Notify(pContainer, &id);

        ::MessageBox(NULL, _T("🎉 INJECTION SUCCESS!\n\nLook at the Tree View NOW.\n(Collapse & Expand if needed)"), _T("VICTORY"), MB_OK);
    }
    catch (...) {
        ::MessageBox(NULL, _T("Crash inside injection logic!"), _T("Fatal"), MB_OK);
    }
}