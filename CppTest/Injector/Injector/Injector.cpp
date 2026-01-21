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

// ==========================================================================
// CONFIGURATION ZONE (唯一需要根据运行时环境修改的地方)
// ==========================================================================

// 🔴 必填：在 Cheat Engine 中搜到的 CHWDataContainer 对象地址
//    (即那个内存里存放着 63 2F 7F E0 的地址)
static void* TARGET_CONTAINER_ADDR = (void*)0x12800590; // <--- 这里填你搜到的新地址

// ==========================================================================
// CONSTANTS (基于 IDA 静态分析确定的事实，无需修改)
// ==========================================================================
// DLL 基址偏移量 (IDA Address - 10000000)
static const uintptr_t OFFSET_GetNewID = 0x471A0;
static const uintptr_t OFFSET_Register = 0x1CF00;
static const uintptr_t OFFSET_Notify   = 0x4A380;

// Payload 路径
const TCHAR* PAYLOAD_PATH = _T("C:\\payload.bin");

// ==========================================================================
// CORE LOGIC (严谨工程实现)
// ==========================================================================

// 存根类，用于创建对象实例
class CModbusSlave : public CObject {
public:
    virtual void Serialize(CArchive& ar);
    static CRuntimeClass* PASCAL GetThisClass();
};

// --------------------------------------------------------------------------
// 汇编级调用封装 (Assembly Wrappers)
// 目的：100% 确保 __thiscall 调用约定正确，防止编译器优化导致的寄存器错误
// --------------------------------------------------------------------------

// Wrapper for GetNewID(this, pSlave)
__declspec(naked) void ASM_Call_GetNewID(void* fn, void* pThis, void* pSlave) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // 将 pThis 放入 ECX (thiscall 核心)
        push [ebp+16]       // 将 pSlave 压栈
        call [ebp+8]        // 调用函数地址
        pop ebp
        ret
    }
}

// Wrapper for Register(this, id) -> returns void**
__declspec(naked) void** ASM_Call_Register(void* fn, void* pThis, int id) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // pThis -> ECX
        push [ebp+16]       // id -> Stack
        call [ebp+8]        // Call fn
        pop ebp
        ret                 // 返回值默认在 EAX 中
    }
}

// Wrapper for Notify(this, int* pID)
__declspec(naked) void ASM_Call_Notify(void* fn, void* pThis, int* pIdPtr) {
    __asm {
        push ebp
        mov ebp, esp
        mov ecx, [ebp+12]   // pThis -> ECX
        push [ebp+16]       // pIdPtr -> Stack
        call [ebp+8]        // Call fn
        pop ebp
        ret
    }
}

// --------------------------------------------------------------------------
// 主执行函数
// --------------------------------------------------------------------------
extern "C" __declspec(dllexport) void RunPoc() {
    AFX_MANAGE_STATE(AfxGetStaticModuleState());

    // 1. 环境校验
    void* pContainer = TARGET_CONTAINER_ADDR;
    if (IsBadReadPtr(pContainer, 4)) {
        ::MessageBox(NULL, _T("Target Address Invalid!\nPlease update line 12 in source code."), _T("Pre-check Failed"), MB_OK);
        return;
    }

    HMODULE hLogic = GetModuleHandle(_T("dllDPLogic.dll"));
    if (!hLogic) {
        ::MessageBox(NULL, _T("dllDPLogic.dll not loaded."), _T("Error"), MB_OK);
        return;
    }
    DWORD_PTR base = (DWORD_PTR)hLogic;

    // 2. 计算函数运行时地址
    void* fnGetNewID = (void*)(base + OFFSET_GetNewID);
    void* fnRegister = (void*)(base + OFFSET_Register);
    void* fnNotify   = (void*)(base + OFFSET_Notify);

    // 3. 加载 Payload (制造对象)
    CFile f;
    if (!f.Open(PAYLOAD_PATH, CFile::modeRead | CFile::typeBinary)) {
        ::MessageBox(NULL, _T("Payload file missing."), _T("Error"), MB_OK);
        return;
    }
    
    ULONGLONG len = f.GetLength();
    BYTE* buf = new BYTE[(size_t)len];
    f.Read(buf, (UINT)len);
    f.Close();
    
    CMemFile mem(buf, (UINT)len);
    CArchive ar(&mem, CArchive::load);

    // 获取类工厂并创建实例
    typedef CRuntimeClass* (*FnGetClass)();
    FnGetClass pfnGetClass = (FnGetClass)GetProcAddress(hLogic, "?GetThisClass@CModbusSlave@@SGPAUCRuntimeClass@@XZ");
    if (!pfnGetClass) {
        delete[] buf;
        ::MessageBox(NULL, _T("Export 'GetThisClass' not found."), _T("Error"), MB_OK);
        return;
    }

    CObject* pSlave = pfnGetClass()->CreateObject();
    try {
        pSlave->Serialize(ar); // 反序列化数据
    } catch(...) {
        delete pSlave; delete[] buf;
        ::MessageBox(NULL, _T("Serialize(Load) Failed."), _T("Error"), MB_OK);
        return;
    }
    ar.Close(); delete[] buf;

    // 4. 执行挂载 (基于逆向分析的确凿逻辑)
    try {
        // Step A: 分配 ID
        // 依据: GetNewID(v35, v3) -> v35 is this
        ASM_Call_GetNewID(fnGetNewID, pContainer, pSlave);

        // Step B: 获取 ID
        // 依据: v26[0] = *((DWORD*)v3 + 6) -> 偏移 24
        int id = *((int*)((char*)pSlave + 24));

        // Step C: 存入哈希表 (修正偏移 +8)
        // 依据: 析构函数中 *(_DWORD *)sub_1001CF00((_DWORD *)this + 2, ...)
        void* pMapThis = (char*)pContainer + 8; // <--- 关键修正点
        
        void** pSlot = ASM_Call_Register(fnRegister, pMapThis, id);
        
        if (pSlot) {
            *pSlot = pSlave; // 将对象指针写入槽位
        } else {
            ::MessageBox(NULL, _T("Register returned NULL pointer."), _T("Error"), MB_OK);
            return; // 此时对象已悬空，暂不处理，优先报错
        }

        // Step D: 通知刷新
        // 依据: sub_1004A380(&v32) -> v32 is ID
        // 注意: 这里用的是 pContainer (不是 +8)，因为原函数中似乎是直接调用的
        // 如果这里还崩，唯一的可能就是 Notify 也需要偏移，但先试这个最可能的
        ASM_Call_Notify(fnNotify, pContainer, &id);

        ::MessageBox(NULL, _T("✅ INJECTION SUCCESS!\nPlease check the Tree View."), _T("Victory"), MB_OK);
    }
    catch (...) {
        ::MessageBox(NULL, _T("CRASHED during injection sequence."), _T("Fatal Error"), MB_OK);
    }
}