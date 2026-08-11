%module xmlpp

%header %{
#include <windows.h>
#include <stdlib.h>
#include "PLinApi.h"
%}


%wrapper %{

%}

%include <stdint.i>
%include <windows.i>


%include "./buffer1.i"
%typemap(in)        (LPSTR  strTextBuff,WORD wBuffSize) = (const void* buffer_data, const size_t buffer_len);
%typemap(typecheck) (LPSTR  strTextBuff,WORD wBuffSize) = (const void* buffer_data, const size_t buffer_len);
%typemap(in)        (void *pBuff,WORD wBuffSize) = (const void* buffer_data, const size_t buffer_len);
%typemap(typecheck) (void *pBuff,WORD wBuffSize) = (const void* buffer_data, const size_t buffer_len);


%include <cpointer.i>

%pointer_class(__int64,INT64_JS)
%pointer_class(int,INT_JS)
%pointer_class(HLINCLIENT,HLINCLIENT_JS)

%include <carrays.i>

%array_class(BYTE, ByteArray);
%array_class(HLINHW,HLINHW_JS)

%include <PLinApi.h>




// %include <cmalloc.i>

//
// %pointer_class(cantp_msg,cantp_msg) 

// %malloc(uint8_t,U8)
// %free(uint8_t,U8)



%inline %{
void LoadDll(const char* path) {
  SetDllDirectory(path);
}
%}



%init %{

extern void CreateTSFN(const Napi::CallbackInfo &info);
extern void FreeTSFN(const Napi::CallbackInfo &info);


do {
  Napi::PropertyDescriptor pd = Napi::PropertyDescriptor::Function("CreateTSFN", CreateTSFN);
  NAPI_CHECK_MAYBE(exports.DefineProperties({
    pd
  }));
} while (0);

do {
  Napi::PropertyDescriptor pd = Napi::PropertyDescriptor::Function("FreeTSFN", FreeTSFN);
  NAPI_CHECK_MAYBE(exports.DefineProperties({
	pd
  }));
} while (0);
%}
