```c
```c
void __thiscall CPOU::Serialize(CPOU *this, unsigned int pExceptionObject)
{
  char v3; // bl
  unsigned int v4; // esi
  int v5; // ecx
  int (__thiscall *v6)(int, unsigned int *); // eax
  int v7; // eax
  int v8; // eax
  int LanguageInfo; // eax
  unsigned int Length; // eax
  const void *v11; // eax
  int v12; // eax
  int v13; // eax
  int v14; // eax
  int v15; // ebx
  const char *v16; // eax
  bool v17; // zf
  unsigned int v18; // eax
  const void *v19; // eax
  unsigned int v20; // eax
  const void *v21; // eax
  int v22; // eax
  int v23; // eax
  int v24; // eax
  int v25; // eax
  const char *v26; // eax
  unsigned int v27; // eax
  const void *v28; // eax
  unsigned int v29; // eax
  const void *v30; // eax
  unsigned int v31; // eax
  const void *v32; // eax
  char v33; // bl
  const char *v34; // eax
  char v35; // bl
  const char *v36; // eax
  char v37; // bl
  const char *v38; // eax
  int v39; // ebx
  const char *v40; // eax
  int v41; // ebx
  const char *v42; // eax
  int v43; // ebx
  const char *v44; // eax
  int v45; // ebx
  const char *v46; // eax
  unsigned int v47; // eax
  const void *v48; // eax
  char v49; // bl
  const char *v50; // eax
  int v51; // ebx
  const char *v52; // eax
  int v53; // eax
  int v54; // ecx
  int v55; // ebx
  const char *v56; // eax
  char v57; // cl
  struct CArchive *v58; // eax
  int v59; // eax
  int v60; // eax
  int v61; // eax
  const char *v62; // eax
  int v63; // eax
  unsigned int v64; // ecx
  int *v65; // eax
  int v66; // ecx
  struct CArchive *v67; // eax
  int v68; // ebx
  int v69; // eax
  int v70; // eax
  int v71; // eax
  int v72; // eax
  const char *v73; // eax
  int v74; // eax
  unsigned int v75; // ecx
  const char *v76; // eax
  int v77; // eax
  unsigned int v78; // ecx
  const char *v79; // eax
  int v80; // eax
  unsigned int v81; // ecx
  const char *v82; // eax
  int v83; // eax
  unsigned int v84; // ecx
  const char *v85; // eax
  int v86; // eax
  unsigned int v87; // ecx
  const char *v88; // eax
  int v89; // eax
  unsigned int v90; // ecx
  const char *v91; // eax
  int v92; // eax
  unsigned int v93; // ecx
  int v94; // eax
  int v95; // ebx
  const char *v96; // eax
  int v97; // eax
  unsigned int v98; // ecx
  const char *v99; // eax
  int v100; // eax
  unsigned int v101; // ecx
  int v102; // eax
  const char *v103; // eax
  unsigned int v104; // ecx
  char v105; // bl
  int v106; // eax
  signed int v107; // ebx
  unsigned int v108; // edx
  int v109; // eax
  int v110; // ebx
  unsigned int v111; // eax
  const void *v112; // eax
  int v113; // eax
  unsigned int v114; // ecx
  CBaseDB **v115; // eax
  CBaseDB *v116; // ecx
  int v117; // ebx
  int v118; // ebx
  CBaseDB *v119; // ebx
  char TypeID; // al
  int v121; // ecx
  int v122; // eax
  unsigned int v123; // ecx
  signed int *v124; // eax
  int v125; // ecx
  int v126; // eax
  unsigned int v127; // ecx
  char *v128; // ecx
  char v129; // al
  CArrayDB *v130; // eax
  CBaseDB *v131; // ebx
  void (__thiscall *v132)(CBaseDB *, unsigned int); // edx
  int v133; // eax
  CBaseDB **v134; // eax
  unsigned int *p_pExceptionObject; // ecx
  CStructDB *v136; // eax
  void (__thiscall *v137)(CBaseDB *, unsigned int); // eax
  int v138; // eax
  CFunctionBlockDB *v139; // eax
  void (__thiscall *v140)(CBaseDB *, unsigned int); // eax
  int v141; // eax
  CBaseDB *v142; // eax
  void (__thiscall *v143)(CBaseDB *, unsigned int); // eax
  int v144; // eax
  CPointerDB *v145; // eax
  void (__thiscall *v146)(CBaseDB *, unsigned int); // eax
  int v147; // eax
  CBaseDB *v148; // eax
  void (__thiscall *v149)(CBaseDB *, unsigned int); // eax
  int v150; // eax
  int v151; // eax
  CMemFile *v152; // ecx
  unsigned __int8 *v153; // ebx
  unsigned int v154; // eax
  CMemFile *v155; // ecx
  unsigned __int8 *v156; // ebx
  void *v157; // ebx
  void *v158; // ebx
  unsigned int v159; // [esp-8h] [ebp-54h]
  unsigned int v160; // [esp-4h] [ebp-50h]
  unsigned int v161; // [esp-4h] [ebp-50h]
  unsigned int v162; // [esp-4h] [ebp-50h]
  unsigned int v163; // [esp-4h] [ebp-50h]
  unsigned int v164; // [esp-4h] [ebp-50h]
  unsigned int v165; // [esp-4h] [ebp-50h]
  unsigned int v166; // [esp-4h] [ebp-50h]
  unsigned int v167; // [esp-4h] [ebp-50h]
  int v168; // [esp-4h] [ebp-50h]
  unsigned int v169; // [esp-4h] [ebp-50h]
  int v170; // [esp+0h] [ebp-4Ch]
  signed int v171; // [esp+14h] [ebp-38h]
  unsigned int v172; // [esp+18h] [ebp-34h] BYREF
  _BYTE v173[4]; // [esp+1Ch] [ebp-30h] BYREF
  CBaseDB *v174; // [esp+20h] [ebp-2Ch] BYREF
  int v175; // [esp+24h] [ebp-28h] BYREF
  _BYTE v176[4]; // [esp+28h] [ebp-24h] BYREF
  _BYTE v177[4]; // [esp+2Ch] [ebp-20h] BYREF
  unsigned int v178; // [esp+30h] [ebp-1Ch] BYREF
  int v179; // [esp+34h] [ebp-18h] BYREF
  _BYTE v180[7]; // [esp+38h] [ebp-14h] BYREF
  char v181; // [esp+3Fh] [ebp-Dh]
  int v182; // [esp+48h] [ebp-4h]

  v3 = 0;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
    v177,
    Default);
  v182 = 0;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
    v173,
    Default);
  v4 = pExceptionObject;
  v5 = *(_DWORD *)(pExceptionObject + 36);
  v6 = *(int (__thiscall **)(int, unsigned int *))(*(_DWORD *)v5 + 24);
  LOBYTE(v182) = 1;
  v7 = v6(v5, &pExceptionObject);
  LOBYTE(v182) = 2;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=(v177, v7);
  LOBYTE(v182) = 1;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&pExceptionObject);
  v8 = ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::Right(v177, &pExceptionObject, 4);
  LOBYTE(v182) = 3;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=(v173, v8);
  LOBYTE(v182) = 1;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&pExceptionObject);
  if ( (unsigned __int8)ATL::CSimpleStringT<char,1>::IsEmpty(v177)
    || !ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CompareNoCase(v173, ".hlf") )
  {
    v3 = 1;
  }
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(v180);
  LOBYTE(v182) = 4;
  LanguageInfo = CAppGlobalFunc::GetLanguageInfo(&pExceptionObject, 0);
  LOBYTE(v182) = 5;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=(v180, LanguageInfo);
  LOBYTE(v182) = 4;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&pExceptionObject);
  if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
  {
    ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
      v176,
      (char *)this + 12);
    v57 = *((_BYTE *)this + 26);
    v181 = *((_BYTE *)this + 24);
    HIBYTE(pExceptionObject) = v57;
    LOBYTE(v182) = 10;
    sub_1000385A((struct CArchive *)v4, (int)this + 12);
    if ( v3 && CAppGlobalFunc::GetSerilizeVersion() >= 0x11 )
    {
      sub_10006BEA((CArchive *)v4, (int)&v172);
      if ( (int)v172 > 0 )
      {
        v174 = (CBaseDB *)v172;
        do
        {
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v179);
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v178);
          LOBYTE(v182) = 12;
          v58 = (struct CArchive *)sub_1000385A((struct CArchive *)v4, (int)&v179);
          sub_1000385A(v58, (int)&v178);
          if ( ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::Find(&v179, &unk_10184414, 0) )
          {
            if ( ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::Find(&v179, &unk_10184410, 0) )
            {
              v170 = ATL::CSimpleStringT<char,1>::operator char const *(&v178);
              v168 = ATL::CSimpleStringT<char,1>::operator char const *(v180);
              v59 = sub_10008EA9(v168);
            }
            else
            {
              v170 = ATL::CSimpleStringT<char,1>::operator char const *(&v178);
              v59 = sub_10008EA9(&unk_10184410);
            }
          }
          else
          {
            v170 = ATL::CSimpleStringT<char,1>::operator char const *(&v178);
            v59 = sub_10008EA9(&unk_10184414);
          }
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=(v59, v170);
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v178);
          LOBYTE(v182) = 10;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v179);
          v174 = (CBaseDB *)((char *)v174 - 1);
        }
        while ( v174 );
      }
      v60 = ATL::CSimpleStringT<char,1>::operator char const *(v180);
      v61 = sub_10008EA9(v60);
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=((char *)this + 16, v61);
      if ( (*(_BYTE *)(v4 + 24) & 1) == 0 )
        goto LABEL_76;
      v63 = *(_DWORD *)(v4 + 40);
      v64 = *(_DWORD *)(v4 + 44);
      if ( v63 + 4 > v64 )
        CArchive::FillBuffer((CArchive *)v4, v63 - v64 + 4);
      v65 = *(int **)(v4 + 40);
      v66 = *v65;
      *(_DWORD *)(v4 + 40) = v65 + 1;
      if ( v66 > 0 )
      {
        v175 = v66;
        do
        {
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v174);
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v178);
          LOBYTE(v182) = 14;
          v67 = (struct CArchive *)sub_1000385A((struct CArchive *)v4, (int)&v174);
          sub_1000385A(v67, (int)&v178);
          v68 = ATL::CSimpleStringT<char,1>::operator char const *(&v178);
          v69 = ATL::CSimpleStringT<char,1>::operator char const *(v180);
          v70 = sub_10008EA9(v69);
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=(v70, v68);
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v178);
          LOBYTE(v182) = 10;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v174);
          --v175;
        }
        while ( v175 );
      }
      v71 = ATL::CSimpleStringT<char,1>::operator char const *(v180);
      v72 = sub_10008EA9(v71);
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=((char *)this + 20, v72);
    }
    else
    {
      sub_1000385A((struct CArchive *)v4, (int)this + 20);
    }
    if ( (*(_BYTE *)(v4 + 24) & 1) == 0 )
    {
      v73 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(4, v73);
    }
    v74 = *(_DWORD *)(v4 + 40);
    v75 = *(_DWORD *)(v4 + 44);
    if ( v74 + 1 > v75 )
      CArchive::FillBuffer((CArchive *)v4, v74 - v75 + 1);
    *((_BYTE *)this + 24) = *(_BYTE *)(*(_DWORD *)(v4 + 40))++;
    if ( (*(_BYTE *)(v4 + 24) & 1) == 0 )
    {
      v76 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(4, v76);
    }
    v77 = *(_DWORD *)(v4 + 40);
    v78 = *(_DWORD *)(v4 + 44);
    if ( v77 + 1 > v78 )
      CArchive::FillBuffer((CArchive *)v4, v77 - v78 + 1);
    *((_BYTE *)this + 25) = *(_BYTE *)(*(_DWORD *)(v4 + 40))++;
    if ( (*(_BYTE *)(v4 + 24) & 1) == 0 )
    {
      v79 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(4, v79);
    }
    v80 = *(_DWORD *)(v4 + 40);
    v81 = *(_DWORD *)(v4 + 44);
    if ( v80 + 1 > v81 )
      CArchive::FillBuffer((CArchive *)v4, v80 - v81 + 1);
    *((_BYTE *)this + 26) = *(_BYTE *)(*(_DWORD *)(v4 + 40))++;
    if ( (*(_BYTE *)(v4 + 24) & 1) == 0 )
    {
      v82 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(4, v82);
    }
    v83 = *(_DWORD *)(v4 + 40);
    v84 = *(_DWORD *)(v4 + 44);
    if ( v83 + 4 > v84 )
      CArchive::FillBuffer((CArchive *)v4, v83 - v84 + 4);
    *((_DWORD *)this + 7) = **(_DWORD **)(v4 + 40);
    *(_DWORD *)(v4 + 40) += 4;
    if ( (*(_BYTE *)(v4 + 24) & 1) == 0 )
    {
      v85 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(4, v85);
    }
    v86 = *(_DWORD *)(v4 + 40);
    v87 = *(_DWORD *)(v4 + 44);
    if ( v86 + 4 > v87 )
      CArchive::FillBuffer((CArchive *)v4, v86 - v87 + 4);
    *((_DWORD *)this + 9) = **(_DWORD **)(v4 + 40);
    *(_DWORD *)(v4 + 40) += 4;
    if ( (*(_BYTE *)(v4 + 24) & 1) == 0 )
    {
      v88 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(4, v88);
    }
    v89 = *(_DWORD *)(v4 + 40);
    v90 = *(_DWORD *)(v4 + 44);
    if ( v89 + 4 > v90 )
      CArchive::FillBuffer((CArchive *)v4, v89 - v90 + 4);
    *((_DWORD *)this + 10) = **(_DWORD **)(v4 + 40);
    *(_DWORD *)(v4 + 40) += 4;
    if ( (*(_BYTE *)(v4 + 24) & 1) == 0 )
    {
      v91 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(4, v91);
    }
    v92 = *(_DWORD *)(v4 + 40);
    v93 = *(_DWORD *)(v4 + 44);
    if ( v92 + 4 > v93 )
      CArchive::FillBuffer((CArchive *)v4, v92 - v93 + 4);
    *((_DWORD *)this + 11) = **(_DWORD **)(v4 + 40);
    *(_DWORD *)(v4 + 40) += 4;
    v94 = sub_1000385A((struct CArchive *)v4, (int)this + 52);
    v95 = v94;
    if ( (*(_BYTE *)(v94 + 24) & 1) == 0 )
    {
      v96 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v94 + 20);
      AfxThrowArchiveException(4, v96);
    }
    v97 = *(_DWORD *)(v94 + 40);
    v98 = *(_DWORD *)(v95 + 44);
    if ( v97 + 1 > v98 )
      CArchive::FillBuffer((CArchive *)v95, v97 - v98 + 1);
    *((_BYTE *)this + 32) = *(_BYTE *)(*(_DWORD *)(v95 + 40))++;
    if ( (*(_BYTE *)(v95 + 24) & 1) == 0 )
    {
      v99 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v95 + 20);
      AfxThrowArchiveException(4, v99);
    }
    v100 = *(_DWORD *)(v95 + 40);
    v101 = *(_DWORD *)(v95 + 44);
    if ( v100 + 4 > v101 )
      CArchive::FillBuffer((CArchive *)v95, v100 - v101 + 4);
    *((_DWORD *)this + 44) = **(_DWORD **)(v95 + 40);
    v102 = *(_DWORD *)(v95 + 40) + 4;
    v17 = (*(_BYTE *)(v95 + 24) & 1) == 0;
    *(_DWORD *)(v95 + 40) = v102;
    if ( v17 )
    {
      v103 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v95 + 20);
      AfxThrowArchiveException(4, v103);
    }
    v104 = *(_DWORD *)(v95 + 44);
    if ( v102 + 4 > v104 )
      CArchive::FillBuffer((CArchive *)v95, v102 - v104 + 4);
    *((_DWORD *)this + 12) = **(_DWORD **)(v95 + 40);
    *(_DWORD *)(v95 + 40) += 4;
    v105 = v181;
    if ( v181 != -1 && HIBYTE(pExceptionObject) != 0xFF )
    {
      v106 = ATL::CSimpleStringT<char,1>::operator char const *((char *)this + 12);
      if ( ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CompareNoCase(v176, v106)
        || v105 != *((_BYTE *)this + 24)
        || HIBYTE(pExceptionObject) != *((_BYTE *)this + 26) )
      {
        pExceptionObject = (unsigned int)this;
        CxxThrowException(&pExceptionObject, (_ThrowInfo *)&_TI3PAVCPOU__);
      }
    }
    LOBYTE(v182) = 4;
    ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(v176);
  }
  else
  {
    Length = ATL::CSimpleStringT<char,1>::GetLength((char *)this + 12);
    AfxWriteStringLength((struct CArchive *)v4, Length, 0);
    v160 = ATL::CSimpleStringT<char,1>::GetLength((char *)this + 12);
    v11 = (const void *)ATL::CSimpleStringT<char,1>::operator char const *((char *)this + 12);
    CArchive::Write((CArchive *)v4, v11, v160);
    if ( v3 )
    {
      pExceptionObject = ATL::CSimpleStringT<char,1>::operator char const *((char *)this + 16);
      v12 = ATL::CSimpleStringT<char,1>::operator char const *(v180);
      v161 = pExceptionObject;
      v13 = sub_10008EA9(v12);
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=(v13, v161);
      if ( (unsigned __int8)ATL::CSimpleStringT<char,1>::IsEmpty((char *)this + 16) )
      {
        v14 = ATL::CSimpleStringT<char,1>::operator char const *(v180);
        sub_10002293(v14);
      }
      v15 = *((_DWORD *)this + 49);
      if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
      {
LABEL_9:
        v16 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
        AfxThrowArchiveException(2, v16);
      }
      if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 4) > *(_DWORD *)(v4 + 44) )
        CArchive::Flush((CArchive *)v4);
      **(_DWORD **)(v4 + 40) = v15;
      *(_DWORD *)(v4 + 40) += 4;
      v17 = *((_DWORD *)this + 49) == 0;
      v178 = -(*((_DWORD *)this + 49) != 0);
      if ( !v17 )
      {
        do
        {
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v179);
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&pExceptionObject);
          LOBYTE(v182) = 7;
          sub_10006AF0(&v178, &v179, &pExceptionObject);
          v18 = ATL::CSimpleStringT<char,1>::GetLength(&v179);
          AfxWriteStringLength((struct CArchive *)v4, v18, 0);
          v162 = ATL::CSimpleStringT<char,1>::GetLength(&v179);
          v19 = (const void *)ATL::CSimpleStringT<char,1>::operator char const *(&v179);
          CArchive::Write((CArchive *)v4, v19, v162);
          v20 = ATL::CSimpleStringT<char,1>::GetLength(&pExceptionObject);
          AfxWriteStringLength((struct CArchive *)v4, v20, 0);
          v163 = ATL::CSimpleStringT<char,1>::GetLength(&pExceptionObject);
          v21 = (const void *)ATL::CSimpleStringT<char,1>::operator char const *(&pExceptionObject);
          CArchive::Write((CArchive *)v4, v21, v163);
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&pExceptionObject);
          LOBYTE(v182) = 4;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v179);
        }
        while ( v178 );
      }
      pExceptionObject = ATL::CSimpleStringT<char,1>::operator char const *((char *)this + 20);
      v22 = ATL::CSimpleStringT<char,1>::operator char const *(v180);
      v164 = pExceptionObject;
      v23 = sub_10008EA9(v22);
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=(v23, v164);
      if ( (unsigned __int8)ATL::CSimpleStringT<char,1>::IsEmpty((char *)this + 20) )
      {
        v24 = ATL::CSimpleStringT<char,1>::operator char const *(v180);
        sub_10002293(v24);
      }
      v25 = ~*(_DWORD *)(v4 + 24);
      pExceptionObject = *((_DWORD *)this + 56);
      if ( (v25 & 1) == 0 )
      {
        v26 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
        AfxThrowArchiveException(2, v26);
      }
      if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 4) > *(_DWORD *)(v4 + 44) )
        CArchive::Flush((CArchive *)v4);
      **(_DWORD **)(v4 + 40) = pExceptionObject;
      *(_DWORD *)(v4 + 40) += 4;
      v17 = *((_DWORD *)this + 56) == 0;
      v178 = -(*((_DWORD *)this + 56) != 0);
      if ( !v17 )
      {
        do
        {
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
            &v179,
            Default);
          LOBYTE(v182) = 8;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
            &pExceptionObject,
            Default);
          LOBYTE(v182) = 9;
          sub_10006AF0(&v178, &v179, &pExceptionObject);
          v27 = ATL::CSimpleStringT<char,1>::GetLength(&v179);
          AfxWriteStringLength((struct CArchive *)v4, v27, 0);
          ATL::CSimpleStringT<char,1>::GetLength(&v179);
          v28 = (const void *)ATL::CSimpleStringT<char,1>::operator char const *(&v179);
          CArchive::Write((CArchive *)v4, v28, v159);
          v29 = ATL::CSimpleStringT<char,1>::GetLength(&pExceptionObject);
          AfxWriteStringLength((struct CArchive *)v4, v29, 0);
          ATL::CSimpleStringT<char,1>::GetLength(&pExceptionObject);
          v30 = (const void *)ATL::CSimpleStringT<char,1>::operator char const *(&pExceptionObject);
          CArchive::Write((CArchive *)v4, v30, v165);
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&pExceptionObject);
          LOBYTE(v182) = 4;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v179);
        }
        while ( v178 );
      }
    }
    else
    {
      v31 = ATL::CSimpleStringT<char,1>::GetLength((char *)this + 20);
      AfxWriteStringLength((struct CArchive *)v4, v31, 0);
      v166 = ATL::CSimpleStringT<char,1>::GetLength((char *)this + 20);
      v32 = (const void *)ATL::CSimpleStringT<char,1>::operator char const *((char *)this + 20);
      CArchive::Write((CArchive *)v4, v32, v166);
    }
    v33 = *((_BYTE *)this + 24);
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v34 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v34);
    }
    if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 1) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    *(_BYTE *)(*(_DWORD *)(v4 + 40))++ = v33;
    v35 = *((_BYTE *)this + 25);
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v36 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v36);
    }
    if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 1) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    *(_BYTE *)(*(_DWORD *)(v4 + 40))++ = v35;
    v37 = *((_BYTE *)this + 26);
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v38 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v38);
    }
    if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 1) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    *(_BYTE *)(*(_DWORD *)(v4 + 40))++ = v37;
    v39 = *((_DWORD *)this + 7);
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v40 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v40);
    }
    if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 4) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    **(_DWORD **)(v4 + 40) = v39;
    *(_DWORD *)(v4 + 40) += 4;
    v41 = *((_DWORD *)this + 9);
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v42 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v42);
    }
    if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 4) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    **(_DWORD **)(v4 + 40) = v41;
    *(_DWORD *)(v4 + 40) += 4;
    v43 = *((_DWORD *)this + 10);
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v44 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v44);
    }
    if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 4) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    **(_DWORD **)(v4 + 40) = v43;
    *(_DWORD *)(v4 + 40) += 4;
    v45 = *((_DWORD *)this + 11);
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v46 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v46);
    }
    if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 4) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    **(_DWORD **)(v4 + 40) = v45;
    *(_DWORD *)(v4 + 40) += 4;
    v47 = ATL::CSimpleStringT<char,1>::GetLength((char *)this + 52);
    AfxWriteStringLength((struct CArchive *)v4, v47, 0);
    v167 = ATL::CSimpleStringT<char,1>::GetLength((char *)this + 52);
    v48 = (const void *)ATL::CSimpleStringT<char,1>::operator char const *((char *)this + 52);
    CArchive::Write((CArchive *)v4, v48, v167);
    v49 = *((_BYTE *)this + 32);
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v50 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v50);
    }
    if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 1) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    *(_BYTE *)(*(_DWORD *)(v4 + 40))++ = v49;
    v51 = *((_DWORD *)this + 44);
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v52 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v52);
    }
    if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 4) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    **(_DWORD **)(v4 + 40) = v51;
    v53 = *(_DWORD *)(v4 + 40) + 4;
    v54 = ~*(_DWORD *)(v4 + 24);
    *(_DWORD *)(v4 + 40) = v53;
    v55 = *((_DWORD *)this + 12);
    if ( (v54 & 1) == 0 )
    {
      v56 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
      AfxThrowArchiveException(2, v56);
    }
    if ( (unsigned int)(v53 + 4) > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    **(_DWORD **)(v4 + 40) = v55;
    *(_DWORD *)(v4 + 40) += 4;
  }
  if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
  {
    v113 = *(_DWORD *)(v4 + 40);
    v114 = *(_DWORD *)(v4 + 44);
    if ( v113 + 4 > v114 )
      CArchive::FillBuffer((CArchive *)v4, v113 - v114 + 4);
    v115 = *(CBaseDB ***)(v4 + 40);
    v116 = *v115;
    *(_DWORD *)(v4 + 40) = v115 + 1;
    if ( (int)v116 > 0 )
    {
      v174 = v116;
      do
      {
        ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
          &pExceptionObject,
          Default);
        LOBYTE(v182) = 15;
        sub_1000385A((struct CArchive *)v4, (int)&pExceptionObject);
        CStringArray::SetAtGrow((char *)this + 76, *((_DWORD *)this + 21), &pExceptionObject);
        LOBYTE(v182) = 4;
        ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&pExceptionObject);
        v174 = (CBaseDB *)((char *)v174 - 1);
      }
      while ( v174 );
    }
  }
  else
  {
    v107 = *((_DWORD *)this + 21);
    v108 = *(_DWORD *)(v4 + 40) + 4;
    v172 = v107;
    if ( v108 > *(_DWORD *)(v4 + 44) )
      CArchive::Flush((CArchive *)v4);
    **(_DWORD **)(v4 + 40) = v107;
    *(_DWORD *)(v4 + 40) += 4;
    v109 = 0;
    pExceptionObject = 0;
    if ( v107 > 0 )
    {
      while ( 1 )
      {
        if ( v109 < 0 || v109 >= *((_DWORD *)this + 21) )
          AfxThrowInvalidArgException();
        v110 = *((_DWORD *)this + 20) + 4 * pExceptionObject;
        v111 = ATL::CSimpleStringT<char,1>::GetLength(v110);
        AfxWriteStringLength((struct CArchive *)v4, v111, 0);
        v169 = ATL::CSimpleStringT<char,1>::GetLength(v110);
        v112 = (const void *)ATL::CSimpleStringT<char,1>::operator char const *(v110);
        CArchive::Write((CArchive *)v4, v112, v169);
        if ( (int)++pExceptionObject >= (int)v172 )
          break;
        v109 = pExceptionObject;
      }
    }
  }
  if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
  {
    v122 = *(_DWORD *)(v4 + 40);
    v123 = *(_DWORD *)(v4 + 44);
    if ( v122 + 4 > v123 )
      CArchive::FillBuffer((CArchive *)v4, v122 - v123 + 4);
    v124 = *(signed int **)(v4 + 40);
    v171 = *v124;
    v125 = *v124;
    *(_DWORD *)(v4 + 40) = v124 + 1;
    v172 = 0;
    if ( v125 <= 0 )
      goto LABEL_194;
    while ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      v126 = *(_DWORD *)(v4 + 40);
      v127 = *(_DWORD *)(v4 + 44);
      if ( v126 + 1 > v127 )
        CArchive::FillBuffer((CArchive *)v4, v126 - v127 + 1);
      v128 = *(char **)(v4 + 40);
      v129 = *v128;
      *(_DWORD *)(v4 + 40) = v128 + 1;
      switch ( v129 )
      {
        case 9:
          v130 = (CArrayDB *)operator new(0xA8u);
          LOBYTE(v182) = 17;
          if ( v130 )
            v131 = CArrayDB::CArrayDB(v130);
          else
            v131 = 0;
          v132 = *(void (__thiscall **)(CBaseDB *, unsigned int))(*(_DWORD *)v131 + 8);
          LOBYTE(v182) = 4;
          v132(v131, v4);
          CBaseDB::GetName(&pExceptionObject);
          LOBYTE(v182) = 18;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(&pExceptionObject);
          v133 = ATL::CSimpleStringT<char,1>::operator char const *(&pExceptionObject);
          v134 = (CBaseDB **)sub_10008F03(v133);
          p_pExceptionObject = &pExceptionObject;
          break;
        case 11:
          v136 = (CStructDB *)operator new(0x8Cu);
          LOBYTE(v182) = 19;
          if ( v136 )
            v131 = CStructDB::CStructDB(v136);
          else
            v131 = 0;
          v137 = *(void (__thiscall **)(CBaseDB *, unsigned int))(*(_DWORD *)v131 + 8);
          LOBYTE(v182) = 4;
          v137(v131, v4);
          CBaseDB::GetName(&v178);
          LOBYTE(v182) = 20;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(&v178);
          v138 = ATL::CSimpleStringT<char,1>::operator char const *(&v178);
          v134 = (CBaseDB **)sub_10008F03(v138);
          p_pExceptionObject = &v178;
          break;
        case 24:
          v139 = (CFunctionBlockDB *)operator new(0x184u);
          LOBYTE(v182) = 21;
          if ( v139 )
            v131 = CFunctionBlockDB::CFunctionBlockDB(v139);
          else
            v131 = 0;
          v140 = *(void (__thiscall **)(CBaseDB *, unsigned int))(*(_DWORD *)v131 + 8);
          LOBYTE(v182) = 4;
          v140(v131, v4);
          CBaseDB::GetName(&v179);
          LOBYTE(v182) = 22;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(&v179);
          v141 = ATL::CSimpleStringT<char,1>::operator char const *(&v179);
          v134 = (CBaseDB **)sub_10008F03(v141);
          p_pExceptionObject = (unsigned int *)&v179;
          break;
        case 10:
          v142 = (CBaseDB *)operator new(0x70u);
          LOBYTE(v182) = 23;
          if ( v142 )
            v131 = CBaseDB::CBaseDB(v142);
          else
            v131 = 0;
          v143 = *(void (__thiscall **)(CBaseDB *, unsigned int))(*(_DWORD *)v131 + 8);
          LOBYTE(v182) = 4;
          v143(v131, v4);
          CBaseDB::GetName(v176);
          LOBYTE(v182) = 24;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(v176);
          v144 = ATL::CSimpleStringT<char,1>::operator char const *(v176);
          v134 = (CBaseDB **)sub_10008F03(v144);
          p_pExceptionObject = (unsigned int *)v176;
          break;
        case 13:
          v145 = (CPointerDB *)operator new(0x74u);
          LOBYTE(v182) = 25;
          if ( v145 )
            v131 = CPointerDB::CPointerDB(v145);
          else
            v131 = 0;
          v146 = *(void (__thiscall **)(CBaseDB *, unsigned int))(*(_DWORD *)v131 + 8);
          LOBYTE(v182) = 4;
          v146(v131, v4);
          CBaseDB::GetName(&v175);
          LOBYTE(v182) = 26;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(&v175);
          v147 = ATL::CSimpleStringT<char,1>::operator char const *(&v175);
          v134 = (CBaseDB **)sub_10008F03(v147);
          p_pExceptionObject = (unsigned int *)&v175;
          break;
        default:
          v148 = (CBaseDB *)operator new(0x70u);
          LOBYTE(v182) = 27;
          if ( v148 )
            v131 = CBaseDB::CBaseDB(v148);
          else
            v131 = 0;
          v149 = *(void (__thiscall **)(CBaseDB *, unsigned int))(*(_DWORD *)v131 + 8);
          LOBYTE(v182) = 4;
          v149(v131, v4);
          CBaseDB::GetName(&v174);
          LOBYTE(v182) = 28;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(&v174);
          v150 = ATL::CSimpleStringT<char,1>::operator char const *(&v174);
          v134 = (CBaseDB **)sub_10008F03(v150);
          p_pExceptionObject = (unsigned int *)&v174;
          break;
      }
      LOBYTE(v182) = 4;
      *v134 = v131;
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(p_pExceptionObject);
      if ( (int)++v172 >= v171 )
        goto LABEL_194;
    }
LABEL_76:
    v62 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v4 + 20);
    AfxThrowArchiveException(4, v62);
  }
  v117 = *(_DWORD *)(*((_DWORD *)this + 24) + 12);
  if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 4) > *(_DWORD *)(v4 + 44) )
    CArchive::Flush((CArchive *)v4);
  **(_DWORD **)(v4 + 40) = v117;
  *(_DWORD *)(v4 + 40) += 4;
  v175 = -(*(_DWORD *)(*((_DWORD *)this + 24) + 12) != 0);
  v118 = v175;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
    v176,
    Default);
  LOBYTE(v182) = 16;
  v174 = 0;
  if ( v118 )
  {
    do
    {
      sub_100012C6(&v175, v176, &v174);
      v119 = v174;
      if ( v174 )
      {
        TypeID = CBaseDB::GetTypeID(v174);
        v121 = ~*(_DWORD *)(v4 + 24);
        LOBYTE(pExceptionObject) = TypeID;
        if ( (v121 & 1) == 0 )
          goto LABEL_9;
        if ( (unsigned int)(*(_DWORD *)(v4 + 40) + 1) > *(_DWORD *)(v4 + 44) )
          CArchive::Flush((CArchive *)v4);
        *(_BYTE *)(*(_DWORD *)(v4 + 40))++ = pExceptionObject;
        (*(void (__thiscall **)(CBaseDB *, unsigned int))(*(_DWORD *)v119 + 8))(v119, v4);
      }
    }
    while ( v175 );
  }
  LOBYTE(v182) = 4;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(v176);
LABEL_194:
  (*(void (__thiscall **)(char *, unsigned int))(*((_DWORD *)this + 14) + 8))((char *)this + 56, v4);
  (*(void (__thiscall **)(_DWORD, unsigned int))(**((_DWORD **)this + 25) + 8))(*((_DWORD *)this + 25), v4);
  if ( CAppGlobalFunc::GetProjectType() == 1 || CPOU::s_bLibTag )
  {
    if ( (*(_BYTE *)(v4 + 24) & 1) != 0 )
    {
      pExceptionObject = 0;
      CArchive::Read((CArchive *)v4, &pExceptionObject, 4u);
      v157 = operator new[](pExceptionObject);
      CArchive::Read((CArchive *)v4, v157, pExceptionObject);
      CMemFile::Attach(*((CMemFile **)this + 29), (unsigned __int8 *)v157, pExceptionObject, 0x400u);
      (*(void (__thiscall **)(_DWORD, void *, unsigned int))(**((_DWORD **)this + 29) + 68))(
        *((_DWORD *)this + 29),
        v157,
        pExceptionObject);
      v178 = 0;
      CArchive::Read((CArchive *)v4, &v178, 4u);
      v158 = operator new[](v178);
      CArchive::Read((CArchive *)v4, v158, v178);
      CMemFile::Attach(*((CMemFile **)this + 32), (unsigned __int8 *)v158, v178, 0x400u);
      (*(void (__thiscall **)(_DWORD, void *, unsigned int))(**((_DWORD **)this + 32) + 68))(
        *((_DWORD *)this + 32),
        v158,
        v178);
    }
    else
    {
      v151 = (*(int (__thiscall **)(_DWORD))(**((_DWORD **)this + 29) + 60))(*((_DWORD *)this + 29));
      v152 = (CMemFile *)*((_DWORD *)this + 29);
      pExceptionObject = v151;
      v153 = CMemFile::Detach(v152);
      CArchive::Write((CArchive *)v4, &pExceptionObject, 4u);
      CArchive::Write((CArchive *)v4, v153, pExceptionObject);
      CArchive::Flush((CArchive *)v4);
      if ( v153 )
        free(v153);
      (*(void (__thiscall **)(_DWORD, _DWORD, _DWORD))(**((_DWORD **)this + 29) + 56))(*((_DWORD *)this + 29), 0, 0);
      v154 = (*(int (__thiscall **)(_DWORD))(**((_DWORD **)this + 32) + 60))(*((_DWORD *)this + 32));
      v155 = (CMemFile *)*((_DWORD *)this + 32);
      v172 = v154;
      v156 = CMemFile::Detach(v155);
      CArchive::Write((CArchive *)v4, &v172, 4u);
      CArchive::Write((CArchive *)v4, v156, v172);
      CArchive::Flush((CArchive *)v4);
      if ( v156 )
        free(v156);
      (*(void (__thiscall **)(_DWORD, _DWORD, _DWORD))(**((_DWORD **)this + 32) + 56))(*((_DWORD *)this + 32), 0, 0);
    }
  }
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(v180);
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(v173);
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(v177);
}
```