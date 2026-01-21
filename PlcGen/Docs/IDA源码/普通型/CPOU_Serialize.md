```c
void __thiscall CPOU::Serialize(CPOU *this, struct CArchive *a2)
{
  __time64_t v3; // rax
  int i; // ebx
  const char *v5; // eax
  _DWORD *v6; // eax
  int StringTOResourceID; // eax
  char v8; // dl
  struct CArchive *v9; // eax
  int v10; // ebx
  const char *v11; // eax
  int v12; // edx
  const char *v13; // eax
  int v14; // ecx
  const char *v15; // eax
  int v16; // eax
  const char *v17; // eax
  int v18; // edx
  const char *v19; // eax
  int v20; // ecx
  const char *v21; // eax
  CArchive *v22; // ebx
  int v23; // eax
  const char *v24; // eax
  int v25; // edx
  const char *v26; // eax
  int v27; // ecx
  const char *v28; // eax
  struct CArchive *v29; // eax
  int v30; // eax
  int v31; // ebx
  const char *v32; // eax
  int v33; // eax
  unsigned int v34; // ecx
  const char *v35; // eax
  int v36; // eax
  unsigned int v37; // ecx
  const char *v38; // eax
  int v39; // eax
  unsigned int v40; // ecx
  const char *v41; // eax
  int v42; // eax
  unsigned int v43; // ecx
  const char *v44; // eax
  int v45; // eax
  unsigned int v46; // ecx
  const char *v47; // eax
  int v48; // eax
  unsigned int v49; // ecx
  int v50; // eax
  int v51; // ebx
  const char *v52; // eax
  int v53; // eax
  unsigned int v54; // ecx
  const char *v55; // eax
  int v56; // eax
  unsigned int v57; // ecx
  int v58; // eax
  bool v59; // zf
  const char *v60; // eax
  unsigned int v61; // ecx
  int v62; // ecx
  unsigned int v63; // eax
  struct CBaseDB *v64; // eax
  int v65; // ebx
  const char *v66; // eax
  int v67; // eax
  unsigned int v68; // ecx
  struct CBaseDB **v69; // eax
  struct CBaseDB *v70; // ecx
  int v71; // ebx
  unsigned int v72; // ebx
  struct CBaseDB *v73; // ebx
  char VarType; // al
  int v75; // ecx
  int v76; // eax
  unsigned int v77; // ecx
  int *v78; // eax
  int v79; // ecx
  int v80; // eax
  unsigned int v81; // ecx
  char *v82; // ecx
  char v83; // al
  void *v84; // eax
  CBaseDB *v85; // ebx
  void (__thiscall *v86)(CBaseDB *, struct CArchive *); // edx
  int v87; // eax
  CBaseDB **v88; // eax
  void *v89; // ecx
  void *v90; // eax
  void (__thiscall *v91)(CBaseDB *, struct CArchive *); // eax
  int v92; // eax
  void *v93; // eax
  void (__thiscall *v94)(CBaseDB *, struct CArchive *); // eax
  int v95; // eax
  void *v96; // eax
  CFunctionBlockDB *v97; // ebx
  void (__thiscall *v98)(CFunctionBlockDB *, struct CArchive *); // eax
  int v99; // eax
  CPOU *v100; // ecx
  void *v101; // eax
  void (__thiscall *v102)(CBaseDB *, struct CArchive *); // eax
  int v103; // eax
  void *v104; // eax
  void (__thiscall *v105)(CBaseDB *, struct CArchive *); // eax
  int v106; // eax
  void *v107; // eax
  void (__thiscall *v108)(CBaseDB *, struct CArchive *); // eax
  int v109; // eax
  struct CBaseDB *v110; // eax
  CMemFile *v111; // ecx
  unsigned __int8 *v112; // ebx
  unsigned int v113; // eax
  CMemFile *v114; // ecx
  unsigned __int8 *v115; // ebx
  void *v116; // ebx
  void *v117; // ebx
  char v118; // bl
  _BYTE *v119; // edx
  int v120; // ecx
  BaseFunc *Buffer; // eax
  const char *v122; // eax
  int v123; // eax
  unsigned int v124; // ecx
  unsigned int v125; // eax
  int v126; // ecx
  int v127; // ecx
  int v128; // edx
  void *v129; // eax
  unsigned int v130; // eax
  int v131; // ecx
  int v132; // ecx
  int v133; // edx
  void *v134; // eax
  int DefString; // eax
  int v136; // ecx
  BaseFunc *v137; // eax
  unsigned int v138; // edx
  unsigned __int8 *v139; // eax
  int *v140; // ecx
  unsigned int v141; // ebx
  unsigned __int8 *v142; // ecx
  int *v143; // edx
  int v144; // eax
  char v145; // bl
  int v146; // edx
  char v147; // bl
  const char *v148; // eax
  int v149; // ebx
  int v150; // eax
  int v151; // eax
  unsigned int v152; // ecx
  int v153; // eax
  const char *v154; // eax
  unsigned int v155; // ecx
  int v156; // [esp-Ch] [ebp-74h] BYREF
  unsigned __int8 *v157; // [esp-8h] [ebp-70h]
  int v158; // [esp-4h] [ebp-6Ch] BYREF
  unsigned __int8 *v159; // [esp+0h] [ebp-68h]
  int v160; // [esp+10h] [ebp-58h] BYREF
  char v161[4]; // [esp+14h] [ebp-54h] BYREF
  char v162[4]; // [esp+18h] [ebp-50h] BYREF
  char v163[4]; // [esp+1Ch] [ebp-4Ch] BYREF
  int v164; // [esp+20h] [ebp-48h] BYREF
  unsigned int v165; // [esp+24h] [ebp-44h] BYREF
  unsigned int v166; // [esp+28h] [ebp-40h] BYREF
  unsigned int v167; // [esp+2Ch] [ebp-3Ch] BYREF
  char v168; // [esp+30h] [ebp-38h]
  struct CBaseDB *v169; // [esp+34h] [ebp-34h] BYREF
  int v170; // [esp+38h] [ebp-30h] BYREF
  int v171; // [esp+3Ch] [ebp-2Ch]
  int v172; // [esp+40h] [ebp-28h]
  int v173; // [esp+44h] [ebp-24h]
  int v174; // [esp+48h] [ebp-20h] BYREF
  int v175; // [esp+4Ch] [ebp-1Ch]
  int v176; // [esp+50h] [ebp-18h]
  void *v177; // [esp+54h] [ebp-14h]
  int v178; // [esp+64h] [ebp-4h]

  if ( (*((_BYTE *)a2 + 24) & 1) != 0 )
  {
    if ( CAppGlobalFunc::GetSerilizeVersion() < 0xF )
    {
      if ( CAppGlobalFunc::GetProjectType() == 1 || CPOU::s_bLibTag || CAppGlobalFunc::GetSerilizeVersion() < 0xA )
        goto LABEL_52;
      v158 = (int)&v169;
    }
    else
    {
      v158 = (int)&v169;
    }
    sub_10012FC0(a2, v158);
LABEL_52:
    v29 = (struct CArchive *)sub_10011B50(a2, (int)this + 8);
    v30 = sub_10011B50(v29, (int)this + 20);
    v31 = v30;
    if ( (*(_BYTE *)(v30 + 24) & 1) == 0 )
    {
      v32 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v30 + 20);
      AfxThrowArchiveException(4, v32);
    }
    v33 = *(_DWORD *)(v30 + 40);
    v34 = *(_DWORD *)(v31 + 44);
    if ( v33 + 1 > v34 )
      CArchive::FillBuffer((CArchive *)v31, v33 - v34 + 1);
    *((_BYTE *)this + 24) = *(_BYTE *)(*(_DWORD *)(v31 + 40))++;
    if ( (*(_BYTE *)(v31 + 24) & 1) == 0 )
    {
      v35 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v31 + 20);
      AfxThrowArchiveException(4, v35);
    }
    v36 = *(_DWORD *)(v31 + 40);
    v37 = *(_DWORD *)(v31 + 44);
    if ( v36 + 1 > v37 )
      CArchive::FillBuffer((CArchive *)v31, v36 - v37 + 1);
    *((_BYTE *)this + 25) = *(_BYTE *)(*(_DWORD *)(v31 + 40))++;
    if ( (*(_BYTE *)(v31 + 24) & 1) == 0 )
    {
      v38 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v31 + 20);
      AfxThrowArchiveException(4, v38);
    }
    v39 = *(_DWORD *)(v31 + 40);
    v40 = *(_DWORD *)(v31 + 44);
    if ( v39 + 4 > v40 )
      CArchive::FillBuffer((CArchive *)v31, v39 - v40 + 4);
    *((_DWORD *)this + 7) = **(_DWORD **)(v31 + 40);
    *(_DWORD *)(v31 + 40) += 4;
    if ( (*(_BYTE *)(v31 + 24) & 1) == 0 )
    {
      v41 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v31 + 20);
      AfxThrowArchiveException(4, v41);
    }
    v42 = *(_DWORD *)(v31 + 40);
    v43 = *(_DWORD *)(v31 + 44);
    if ( v42 + 4 > v43 )
      CArchive::FillBuffer((CArchive *)v31, v42 - v43 + 4);
    *((_DWORD *)this + 9) = **(_DWORD **)(v31 + 40);
    *(_DWORD *)(v31 + 40) += 4;
    if ( (*(_BYTE *)(v31 + 24) & 1) == 0 )
    {
      v44 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v31 + 20);
      AfxThrowArchiveException(4, v44);
    }
    v45 = *(_DWORD *)(v31 + 40);
    v46 = *(_DWORD *)(v31 + 44);
    if ( v45 + 4 > v46 )
      CArchive::FillBuffer((CArchive *)v31, v45 - v46 + 4);
    *((_DWORD *)this + 10) = **(_DWORD **)(v31 + 40);
    *(_DWORD *)(v31 + 40) += 4;
    if ( (*(_BYTE *)(v31 + 24) & 1) == 0 )
    {
      v47 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v31 + 20);
      AfxThrowArchiveException(4, v47);
    }
    v48 = *(_DWORD *)(v31 + 40);
    v49 = *(_DWORD *)(v31 + 44);
    if ( v48 + 4 > v49 )
      CArchive::FillBuffer((CArchive *)v31, v48 - v49 + 4);
    *((_DWORD *)this + 11) = **(_DWORD **)(v31 + 40);
    *(_DWORD *)(v31 + 40) += 4;
    v50 = sub_10011B50((struct CArchive *)v31, (int)this + 52);
    v51 = v50;
    if ( (*(_BYTE *)(v50 + 24) & 1) == 0 )
    {
      v52 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v50 + 20);
      AfxThrowArchiveException(4, v52);
    }
    v53 = *(_DWORD *)(v50 + 40);
    v54 = *(_DWORD *)(v51 + 44);
    if ( v53 + 1 > v54 )
      CArchive::FillBuffer((CArchive *)v51, v53 - v54 + 1);
    *((_BYTE *)this + 32) = *(_BYTE *)(*(_DWORD *)(v51 + 40))++;
    if ( (*(_BYTE *)(v51 + 24) & 1) == 0 )
    {
      v55 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v51 + 20);
      AfxThrowArchiveException(4, v55);
    }
    v56 = *(_DWORD *)(v51 + 40);
    v57 = *(_DWORD *)(v51 + 44);
    if ( v56 + 4 > v57 )
      CArchive::FillBuffer((CArchive *)v51, v56 - v57 + 4);
    *((_DWORD *)this + 43) = **(_DWORD **)(v51 + 40);
    v58 = *(_DWORD *)(v51 + 40) + 4;
    v59 = (*(_BYTE *)(v51 + 24) & 1) == 0;
    *(_DWORD *)(v51 + 40) = v58;
    if ( v59 )
    {
      v60 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v51 + 20);
      AfxThrowArchiveException(4, v60);
    }
    v61 = *(_DWORD *)(v51 + 44);
    if ( v58 + 4 > v61 )
      CArchive::FillBuffer((CArchive *)v51, v58 - v61 + 4);
    *((_DWORD *)this + 12) = **(_DWORD **)(v51 + 40);
    *(_DWORD *)(v51 + 40) += 4;
    if ( CAppGlobalFunc::GetSerilizeVersion() < 0x44 )
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=((char *)this + 56, (char *)this + 52);
    else
      sub_10011B50(a2, (int)this + 56);
    if ( !(unsigned __int8)ATL::CSimpleStringT<char,1>::IsEmpty((char *)this + 20) )
    {
      v158 = v62;
      v169 = (struct CBaseDB *)&v158;
      sub_10020BA0(&v158, "POUDesc::::", (char *)this + 20);
      CAppGlobalFunc::WriteLog(v158, v159);
    }
    goto LABEL_93;
  }
  v3 = _time64(0);
  v177 = (void *)HIDWORD(v3);
  for ( i = v3; !i; ++i )
    ;
  if ( (*((_BYTE *)a2 + 24) & 1) != 0 )
  {
LABEL_5:
    v5 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *((char *)a2 + 20);
    AfxThrowArchiveException(2, v5);
  }
  if ( (unsigned int)(*((_DWORD *)a2 + 10) + 4) > *((_DWORD *)a2 + 11) )
    CArchive::Flush(a2);
  v6 = (_DWORD *)*((_DWORD *)a2 + 10);
  v158 = (int)this + 56;
  v157 = (unsigned __int8 *)this + 52;
  v156 = (int)this + 56;
  *v6 = i;
  *((_DWORD *)a2 + 10) += 4;
  v169 = (struct CBaseDB *)&v156;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
    &v156,
    (char *)this + 20);
  StringTOResourceID = CAppGlobalFunc::GetStringTOResourceID(&v167, v156);
  v8 = *((_BYTE *)this + 24);
  v156 = StringTOResourceID;
  v178 = 0;
  v168 = v8;
  v9 = (struct CArchive *)sub_10010320(a2, (int)this + 8);
  v10 = sub_10010320(v9, v156);
  if ( (*(_BYTE *)(v10 + 24) & 1) != 0 )
  {
    v11 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v10 + 20);
    AfxThrowArchiveException(2, v11);
  }
  if ( (unsigned int)(*(_DWORD *)(v10 + 40) + 1) > *(_DWORD *)(v10 + 44) )
    CArchive::Flush((CArchive *)v10);
  *(_BYTE *)(*(_DWORD *)(v10 + 40))++ = v168;
  v12 = ~*(_DWORD *)(v10 + 24);
  v168 = *((_BYTE *)this + 25);
  if ( (v12 & 1) == 0 )
  {
    v13 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v10 + 20);
    AfxThrowArchiveException(2, v13);
  }
  if ( (unsigned int)(*(_DWORD *)(v10 + 40) + 1) > *(_DWORD *)(v10 + 44) )
    CArchive::Flush((CArchive *)v10);
  *(_BYTE *)(*(_DWORD *)(v10 + 40))++ = v168;
  v14 = ~*(_DWORD *)(v10 + 24);
  v169 = (struct CBaseDB *)*((_DWORD *)this + 7);
  if ( (v14 & 1) == 0 )
  {
    v15 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v10 + 20);
    AfxThrowArchiveException(2, v15);
  }
  if ( (unsigned int)(*(_DWORD *)(v10 + 40) + 4) > *(_DWORD *)(v10 + 44) )
    CArchive::Flush((CArchive *)v10);
  **(_DWORD **)(v10 + 40) = v169;
  *(_DWORD *)(v10 + 40) += 4;
  v16 = ~*(_DWORD *)(v10 + 24);
  v169 = (struct CBaseDB *)*((_DWORD *)this + 9);
  if ( (v16 & 1) == 0 )
  {
    v17 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v10 + 20);
    AfxThrowArchiveException(2, v17);
  }
  if ( (unsigned int)(*(_DWORD *)(v10 + 40) + 4) > *(_DWORD *)(v10 + 44) )
    CArchive::Flush((CArchive *)v10);
  **(_DWORD **)(v10 + 40) = v169;
  *(_DWORD *)(v10 + 40) += 4;
  v18 = ~*(_DWORD *)(v10 + 24);
  v169 = (struct CBaseDB *)*((_DWORD *)this + 10);
  if ( (v18 & 1) == 0 )
  {
    v19 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v10 + 20);
    AfxThrowArchiveException(2, v19);
  }
  if ( (unsigned int)(*(_DWORD *)(v10 + 40) + 4) > *(_DWORD *)(v10 + 44) )
    CArchive::Flush((CArchive *)v10);
  **(_DWORD **)(v10 + 40) = v169;
  *(_DWORD *)(v10 + 40) += 4;
  v20 = ~*(_DWORD *)(v10 + 24);
  v169 = (struct CBaseDB *)*((_DWORD *)this + 11);
  if ( (v20 & 1) == 0 )
  {
    v21 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *(v10 + 20);
    AfxThrowArchiveException(2, v21);
  }
  if ( (unsigned int)(*(_DWORD *)(v10 + 40) + 4) > *(_DWORD *)(v10 + 44) )
    CArchive::Flush((CArchive *)v10);
  **(_DWORD **)(v10 + 40) = v169;
  *(_DWORD *)(v10 + 40) += 4;
  v22 = (CArchive *)sub_10010320((struct CArchive *)v10, (int)v157);
  v23 = ~*((_DWORD *)v22 + 6);
  v168 = *((_BYTE *)this + 32);
  if ( (v23 & 1) == 0 )
  {
    v24 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *((char *)v22 + 20);
    AfxThrowArchiveException(2, v24);
  }
  if ( (unsigned int)(*((_DWORD *)v22 + 10) + 1) > *((_DWORD *)v22 + 11) )
    CArchive::Flush(v22);
  *(_BYTE *)(*((_DWORD *)v22 + 10))++ = v168;
  v25 = ~*((_DWORD *)v22 + 6);
  v169 = (struct CBaseDB *)*((_DWORD *)this + 43);
  if ( (v25 & 1) == 0 )
  {
    v26 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *((char *)v22 + 20);
    AfxThrowArchiveException(2, v26);
  }
  if ( (unsigned int)(*((_DWORD *)v22 + 10) + 4) > *((_DWORD *)v22 + 11) )
    CArchive::Flush(v22);
  **((_DWORD **)v22 + 10) = v169;
  *((_DWORD *)v22 + 10) += 4;
  v27 = ~*((_DWORD *)v22 + 6);
  v169 = (struct CBaseDB *)*((_DWORD *)this + 12);
  if ( (v27 & 1) == 0 )
  {
    v28 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *((char *)v22 + 20);
    AfxThrowArchiveException(2, v28);
  }
  if ( (unsigned int)(*((_DWORD *)v22 + 10) + 4) > *((_DWORD *)v22 + 11) )
    CArchive::Flush(v22);
  **((_DWORD **)v22 + 10) = v169;
  *((_DWORD *)v22 + 10) += 4;
  sub_10010320(v22, v158);
  v178 = -1;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v167);
LABEL_93:
  if ( (*((_BYTE *)a2 + 24) & 1) != 0 )
  {
    v67 = *((_DWORD *)a2 + 10);
    v68 = *((_DWORD *)a2 + 11);
    if ( v67 + 4 > v68 )
      CArchive::FillBuffer(a2, v67 - v68 + 4);
    v69 = (struct CBaseDB **)*((_DWORD *)a2 + 10);
    v70 = *v69;
    *((_DWORD *)a2 + 10) = v69 + 1;
    if ( (int)v70 > 0 )
    {
      v169 = v70;
      do
      {
        ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
          &v166,
          Default);
        v178 = 1;
        sub_10011B50(a2, (int)&v166);
        CStringArray::SetAtGrow((char *)this + 80, *((_DWORD *)this + 22), &v166);
        v178 = -1;
        ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v166);
        v169 = (struct CBaseDB *)((char *)v169 - 1);
      }
      while ( v169 );
    }
  }
  else
  {
    v63 = *((_DWORD *)a2 + 10) + 4;
    v169 = (struct CBaseDB *)*((_DWORD *)this + 22);
    if ( v63 > *((_DWORD *)a2 + 11) )
      CArchive::Flush(a2);
    v64 = v169;
    **((_DWORD **)a2 + 10) = v169;
    *((_DWORD *)a2 + 10) += 4;
    v65 = 0;
    if ( (int)v64 > 0 )
    {
      do
      {
        if ( v65 < 0 || v65 >= *((_DWORD *)this + 22) )
          AfxThrowInvalidArgException();
        sub_10010320(a2, *((_DWORD *)this + 21) + 4 * v65++);
      }
      while ( v65 < (int)v169 );
    }
  }
  if ( (*((_BYTE *)a2 + 24) & 1) != 0 )
  {
    v76 = *((_DWORD *)a2 + 10);
    v77 = *((_DWORD *)a2 + 11);
    if ( v76 + 4 > v77 )
      CArchive::FillBuffer(a2, v76 - v77 + 4);
    v78 = (int *)*((_DWORD *)a2 + 10);
    v160 = *v78;
    v79 = v160;
    *((_DWORD *)a2 + 10) = v78 + 1;
    v164 = 0;
    if ( v79 <= 0 )
    {
LABEL_165:
      CPOU::SavePOUTmpVarBak(this);
      goto LABEL_166;
    }
    while ( 1 )
    {
      if ( (*((_BYTE *)a2 + 24) & 1) == 0 )
      {
        v66 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *((char *)a2 + 20);
        AfxThrowArchiveException(4, v66);
      }
      v80 = *((_DWORD *)a2 + 10);
      v81 = *((_DWORD *)a2 + 11);
      if ( v80 + 1 > v81 )
        CArchive::FillBuffer(a2, v80 - v81 + 1);
      v82 = (char *)*((_DWORD *)a2 + 10);
      v83 = *v82;
      *((_DWORD *)a2 + 10) = v82 + 1;
      switch ( v83 )
      {
        case 8:
          v84 = operator new(0x7Cu);
          v177 = v84;
          v178 = 3;
          if ( v84 )
            v85 = CStringDB::CStringDB((CStringDB *)v84);
          else
            v85 = 0;
          v86 = *(void (__thiscall **)(CBaseDB *, struct CArchive *))(*(_DWORD *)v85 + 8);
          v178 = -1;
          v86(v85, a2);
          CBaseDB::GetName(&v166);
          v178 = 4;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(&v166);
          v87 = ATL::CSimpleStringT<char,1>::operator char const *(&v166);
          v88 = (CBaseDB **)sub_10036090(v87);
          v89 = &v166;
          break;
        case 9:
          v90 = operator new(0xA4u);
          v177 = v90;
          v178 = 5;
          if ( v90 )
            v85 = CArrayDB::CArrayDB((CArrayDB *)v90);
          else
            v85 = 0;
          v91 = *(void (__thiscall **)(CBaseDB *, struct CArchive *))(*(_DWORD *)v85 + 8);
          v178 = -1;
          v91(v85, a2);
          CBaseDB::GetName(&v165);
          v178 = 6;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(&v165);
          v92 = ATL::CSimpleStringT<char,1>::operator char const *(&v165);
          v88 = (CBaseDB **)sub_10036090(v92);
          v89 = &v165;
          break;
        case 11:
          v93 = operator new(0x74u);
          v177 = v93;
          v178 = 7;
          if ( v93 )
            v85 = CStructDB::CStructDB((CStructDB *)v93);
          else
            v85 = 0;
          v94 = *(void (__thiscall **)(CBaseDB *, struct CArchive *))(*(_DWORD *)v85 + 8);
          v178 = -1;
          v94(v85, a2);
          CBaseDB::GetName(v161);
          v178 = 8;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(v161);
          v95 = ATL::CSimpleStringT<char,1>::operator char const *(v161);
          v88 = (CBaseDB **)sub_10036090(v95);
          v89 = v161;
          break;
        case 24:
          v96 = operator new(0x118u);
          v177 = v96;
          v178 = 9;
          if ( v96 )
            v97 = CFunctionBlockDB::CFunctionBlockDB((CFunctionBlockDB *)v96);
          else
            v97 = 0;
          v98 = *(void (__thiscall **)(CFunctionBlockDB *, struct CArchive *))(*(_DWORD *)v97 + 8);
          v178 = -1;
          v98(v97, a2);
          CBaseDB::GetName(v162);
          v178 = 10;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(v162);
          v99 = ATL::CSimpleStringT<char,1>::operator char const *(v162);
          *(_DWORD *)sub_10036090(v99) = v97;
          if ( CPOU::GetPOUType(this) == 1 && CPOU::GetPOULanguage(v100) == 2 )
            *((_DWORD *)this + 45) = v97;
          v89 = v162;
          goto LABEL_164;
        case 10:
          v101 = operator new(0x58u);
          v177 = v101;
          v178 = 11;
          if ( v101 )
            v85 = CBaseDB::CBaseDB((CBaseDB *)v101);
          else
            v85 = 0;
          v102 = *(void (__thiscall **)(CBaseDB *, struct CArchive *))(*(_DWORD *)v85 + 8);
          v178 = -1;
          v102(v85, a2);
          CBaseDB::GetName(v163);
          v178 = 12;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(v163);
          v103 = ATL::CSimpleStringT<char,1>::operator char const *(v163);
          v88 = (CBaseDB **)sub_10036090(v103);
          v89 = v163;
          break;
        case 13:
          v104 = operator new(0x60u);
          v177 = v104;
          v178 = 13;
          if ( v104 )
            v85 = CPointerDB::CPointerDB((CPointerDB *)v104);
          else
            v85 = 0;
          v105 = *(void (__thiscall **)(CBaseDB *, struct CArchive *))(*(_DWORD *)v85 + 8);
          v178 = -1;
          v105(v85, a2);
          CBaseDB::GetName(&v167);
          v178 = 14;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(&v167);
          v106 = ATL::CSimpleStringT<char,1>::operator char const *(&v167);
          v88 = (CBaseDB **)sub_10036090(v106);
          v89 = &v167;
          break;
        default:
          v107 = operator new(0x58u);
          v177 = v107;
          v178 = 15;
          if ( v107 )
            v85 = CBaseDB::CBaseDB((CBaseDB *)v107);
          else
            v85 = 0;
          v108 = *(void (__thiscall **)(CBaseDB *, struct CArchive *))(*(_DWORD *)v85 + 8);
          v178 = -1;
          v108(v85, a2);
          CBaseDB::GetName(&v169);
          v178 = 16;
          ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::MakeUpper(&v169);
          v109 = ATL::CSimpleStringT<char,1>::operator char const *(&v169);
          v88 = (CBaseDB **)sub_10036090(v109);
          v89 = &v169;
          break;
      }
      *v88 = v85;
LABEL_164:
      v178 = -1;
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(v89);
      if ( ++v164 >= v160 )
        goto LABEL_165;
    }
  }
  v71 = *(_DWORD *)(*((_DWORD *)this + 25) + 12);
  if ( (unsigned int)(*((_DWORD *)a2 + 10) + 4) > *((_DWORD *)a2 + 11) )
    CArchive::Flush(a2);
  **((_DWORD **)a2 + 10) = v71;
  *((_DWORD *)a2 + 10) += 4;
  v167 = -(*(_DWORD *)(*((_DWORD *)this + 25) + 12) != 0);
  v72 = v167;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
    v163,
    Default);
  v178 = 2;
  v169 = 0;
  if ( v72 )
  {
    do
    {
      sub_10066210(&v167, v163, &v169);
      v73 = v169;
      if ( v169 )
      {
        VarType = CAppGlobalFunc::GetVarType(v169);
        v75 = ~*((_DWORD *)a2 + 6);
        v168 = VarType;
        if ( (v75 & 1) == 0 )
          goto LABEL_5;
        if ( (unsigned int)(*((_DWORD *)a2 + 10) + 1) > *((_DWORD *)a2 + 11) )
          CArchive::Flush(a2);
        *(_BYTE *)(*((_DWORD *)a2 + 10))++ = v168;
        (*(void (__thiscall **)(struct CBaseDB *, struct CArchive *))(*(_DWORD *)v73 + 8))(v73, a2);
      }
    }
    while ( v167 );
  }
  v178 = -1;
  ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(v163);
LABEL_166:
  (*(void (__thiscall **)(char *, struct CArchive *))(*((_DWORD *)this + 15) + 8))((char *)this + 60, a2);
  (*(void (__thiscall **)(_DWORD, struct CArchive *))(**((_DWORD **)this + 27) + 8))(*((_DWORD *)this + 27), a2);
  if ( CAppGlobalFunc::GetProjectType() == 1 || CPOU::s_bLibTag )
  {
    if ( (*((_BYTE *)a2 + 24) & 1) != 0 )
    {
      v165 = 0;
      CArchive::Read(a2, &v165, 4u);
      v116 = operator new[](v165);
      CArchive::Read(a2, v116, v165);
      CMemFile::Attach(*((CMemFile **)this + 31), (unsigned __int8 *)v116, v165, 0x400u);
      (*(void (__thiscall **)(_DWORD, void *, unsigned int))(**((_DWORD **)this + 31) + 68))(
        *((_DWORD *)this + 31),
        v116,
        v165);
      v166 = 0;
      CArchive::Read(a2, &v166, 4u);
      v117 = operator new[](v166);
      CArchive::Read(a2, v117, v166);
      CMemFile::Attach(*((CMemFile **)this + 32), (unsigned __int8 *)v117, v166, 0x400u);
      (*(void (__thiscall **)(_DWORD, void *, unsigned int))(**((_DWORD **)this + 32) + 68))(
        *((_DWORD *)this + 32),
        v117,
        v166);
    }
    else
    {
      v110 = (struct CBaseDB *)(*(int (__thiscall **)(_DWORD))(**((_DWORD **)this + 31) + 60))(*((_DWORD *)this + 31));
      v111 = (CMemFile *)*((_DWORD *)this + 31);
      v169 = v110;
      v112 = CMemFile::Detach(v111);
      CArchive::Write(a2, &v169, 4u);
      CArchive::Write(a2, v112, (unsigned int)v169);
      CArchive::Flush(a2);
      if ( v112 )
        free(v112);
      (*(void (__thiscall **)(_DWORD, _DWORD, _DWORD))(**((_DWORD **)this + 31) + 56))(*((_DWORD *)this + 31), 0, 0);
      v113 = (*(int (__thiscall **)(_DWORD))(**((_DWORD **)this + 32) + 60))(*((_DWORD *)this + 32));
      v114 = (CMemFile *)*((_DWORD *)this + 32);
      v167 = v113;
      v115 = CMemFile::Detach(v114);
      CArchive::Write(a2, &v167, 4u);
      CArchive::Write(a2, v115, v167);
      CArchive::Flush(a2);
      if ( v115 )
        free(v115);
      (*(void (__thiscall **)(_DWORD, _DWORD, _DWORD))(**((_DWORD **)this + 32) + 56))(*((_DWORD *)this + 32), 0, 0);
    }
  }
  if ( (*((_BYTE *)a2 + 24) & 1) != 0 )
  {
    if ( CAppGlobalFunc::GetSerilizeVersion() >= 4 )
    {
      v170 = 0;
      v171 = 0;
      v172 = 0;
      v173 = 0;
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
        &v164,
        Default);
      v159 = (unsigned __int8 *)&v170;
      v158 = 0;
      v157 = 0;
      v178 = 17;
      Buffer = (BaseFunc *)ATL::CSimpleStringT<char,1>::GetBuffer(&v164);
      BaseFunc::GetPassword128(Buffer, v157, v158, v159);
      if ( CAppGlobalFunc::GetSerilizeVersion() < 0x3C )
      {
        v174 = 0;
        v175 = 0;
        v176 = 0;
        v177 = 0;
        CArchive::Read(a2, &v174, 0x10u);
        v125 = 16;
        v126 = 0;
        while ( *(int *)((char *)&v170 + v126) == *(int *)((char *)&v174 + v126) )
        {
          v125 -= 4;
          v126 += 4;
          if ( v125 < 4 )
            goto LABEL_191;
        }
        v127 = v175;
        v128 = v176;
        *((_DWORD *)this + 48) = v174;
        v129 = v177;
        *((_DWORD *)this + 49) = v127;
        *((_DWORD *)this + 50) = v128;
        *((_DWORD *)this + 51) = v129;
LABEL_191:
        v174 = 0;
        v175 = 0;
        v176 = 0;
        v177 = 0;
        CArchive::Read(a2, &v174, 0x10u);
        v130 = 16;
        v131 = 0;
        while ( *(int *)((char *)&v170 + v131) == *(int *)((char *)&v174 + v131) )
        {
          v130 -= 4;
          v131 += 4;
          if ( v130 < 4 )
            goto LABEL_196;
        }
        v132 = v175;
        v133 = v176;
        *((_DWORD *)this + 52) = v174;
        v134 = v177;
        *((_DWORD *)this + 53) = v132;
        *((_DWORD *)this + 54) = v133;
        *((_DWORD *)this + 55) = v134;
      }
      else
      {
        if ( (*((_BYTE *)a2 + 24) & 1) == 0 )
        {
          v122 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *((char *)a2 + 20);
          AfxThrowArchiveException(4, v122);
        }
        v123 = *((_DWORD *)a2 + 10);
        v124 = *((_DWORD *)a2 + 11);
        if ( v123 + 1 > v124 )
          CArchive::FillBuffer(a2, v123 - v124 + 1);
        *((_BYTE *)this + 184) = *(_BYTE *)(*((_DWORD *)a2 + 10))++;
        CArchive::Read(a2, (char *)this + 192, 0x10u);
        CArchive::Read(a2, (char *)this + 208, 0x10u);
      }
LABEL_196:
      v158 = *((unsigned __int8 *)this + 184);
      v157 = (unsigned __int8 *)&v160;
      v170 = 0;
      v171 = 0;
      v172 = 0;
      v173 = 0;
      CAppGlobalFunc::GetStation();
      DefString = CStation::GetDefString(v157, v158);
      LOBYTE(v178) = 18;
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::operator=(&v164, DefString);
      LOBYTE(v178) = 17;
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v160);
      v136 = *((unsigned __int8 *)this + 184);
      v159 = (unsigned __int8 *)&v170;
      v158 = v136;
      v157 = 0;
      v137 = (BaseFunc *)ATL::CSimpleStringT<char,1>::GetBuffer(&v164);
      BaseFunc::GetPassword128(v137, v157, v158, v159);
      v138 = 16;
      v139 = (unsigned __int8 *)this + 192;
      v140 = &v170;
      while ( *v140 == *(_DWORD *)v139 )
      {
        v138 -= 4;
        v139 += 4;
        ++v140;
        if ( v138 < 4 )
        {
          v141 = 0;
          goto LABEL_200;
        }
      }
      v149 = (unsigned __int8)*v140 - *v139;
      if ( !v149 )
      {
        v149 = *((unsigned __int8 *)v140 + 1) - v139[1];
        if ( !v149 )
        {
          v149 = *((unsigned __int8 *)v140 + 2) - v139[2];
          if ( !v149 )
            v149 = *((unsigned __int8 *)v140 + 3) - v139[3];
        }
      }
      v141 = (v149 >> 31) | 1;
LABEL_200:
      v167 = v141;
      v169 = (struct CBaseDB *)16;
      v142 = (unsigned __int8 *)this + 208;
      v143 = &v170;
      while ( *v143 == *(_DWORD *)v142 )
      {
        v142 += 4;
        ++v143;
        v169 = (struct CBaseDB *)((char *)v169 - 4);
        if ( (unsigned int)v169 < 4 )
        {
          v144 = 0;
          goto LABEL_204;
        }
      }
      v150 = (unsigned __int8)*v143 - *v142;
      if ( !v150 )
      {
        v150 = *((unsigned __int8 *)v143 + 1) - v142[1];
        if ( !v150 )
        {
          v150 = *((unsigned __int8 *)v143 + 2) - v142[2];
          if ( !v150 )
            v150 = *((unsigned __int8 *)v143 + 3) - v142[3];
        }
      }
      v141 = v167;
      v144 = (v150 >> 31) | 1;
LABEL_204:
      if ( v141 || v144 )
        *((_BYTE *)this + 224) = 1;
      v178 = -1;
      ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::~CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(&v164);
    }
  }
  else
  {
    v118 = *((_BYTE *)this + 184);
    if ( (unsigned int)(*((_DWORD *)a2 + 10) + 1) > *((_DWORD *)a2 + 11) )
      CArchive::Flush(a2);
    v119 = (_BYTE *)*((_DWORD *)a2 + 10);
    v158 = 16;
    *v119 = v118;
    ++*((_DWORD *)a2 + 10);
    CArchive::Write(a2, (char *)this + 192, v158);
    CArchive::Write(a2, (char *)this + 208, 0x10u);
  }
  if ( (*((_BYTE *)a2 + 24) & 1) != 0 )
  {
    v158 = v120;
    v177 = &v158;
    ATL::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>::CStringT<char,StrTraitMFC_DLL<char,ATL::ChTraitsCRT<char>>>(
      &v158,
      (char *)this + 8);
    CPOU::SetPOUShowName(v158);
  }
  if ( CAppGlobalFunc::GetSerilizeVersion() >= 0x29 )
  {
    if ( (*((_BYTE *)a2 + 24) & 1) != 0 )
    {
      v151 = *((_DWORD *)a2 + 10);
      v152 = *((_DWORD *)a2 + 11);
      if ( v151 + 1 > v152 )
        CArchive::FillBuffer(a2, v151 - v152 + 1);
      *((_BYTE *)this + 253) = *(_BYTE *)(*((_DWORD *)a2 + 10))++;
      v153 = *((_DWORD *)a2 + 10);
      if ( (*((_BYTE *)a2 + 24) & 1) == 0 )
      {
        v154 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *((char *)a2 + 20);
        AfxThrowArchiveException(4, v154);
      }
      v155 = *((_DWORD *)a2 + 11);
      if ( v153 + 1 > v155 )
        CArchive::FillBuffer(a2, v153 - v155 + 1);
      *((_BYTE *)this + 254) = *(_BYTE *)(*((_DWORD *)a2 + 10))++;
    }
    else
    {
      v145 = *((_BYTE *)this + 253);
      if ( (unsigned int)(*((_DWORD *)a2 + 10) + 1) > *((_DWORD *)a2 + 11) )
        CArchive::Flush(a2);
      **((_BYTE **)a2 + 10) = v145;
      v146 = *((_DWORD *)a2 + 6);
      ++*((_DWORD *)a2 + 10);
      v147 = *((_BYTE *)this + 254);
      if ( (v146 & 1) != 0 )
      {
        v148 = (const char *)ATL::CSimpleStringT<char,1>::operator char const *((char *)a2 + 20);
        AfxThrowArchiveException(2, v148);
      }
      if ( (unsigned int)(*((_DWORD *)a2 + 10) + 1) > *((_DWORD *)a2 + 11) )
        CArchive::Flush(a2);
      *(_BYTE *)(*((_DWORD *)a2 + 10))++ = v147;
    }
  }
}
```