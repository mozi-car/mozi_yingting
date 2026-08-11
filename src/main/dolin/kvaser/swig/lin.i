%module xmlpp

%header %{
#include <windows.h>
#include <stdlib.h>
#include "linlib.h"
%}


%wrapper %{

%}

%include <stdint.i>
%include <windows.i>


%include "./buffer1.i"
%typemap(in)        (void *buffer, size_t bufsize) = (const void* buffer_data, const size_t buffer_len);
%typemap(typecheck) (void *buffer, size_t bufsize) = (const void* buffer_data, const size_t buffer_len);

%typemap(in)        (const void *msg,unsigned int dlc) = (const void* buffer_data, const size_t buffer_len);
%typemap(typecheck) (const void *msg,unsigned int dlc) = (const void* buffer_data, const size_t buffer_len);


%include <cpointer.i>

%pointer_class(unsigned char,CharPointer)
%pointer_class(int,IntPointer)
// %pointer_class(int,INT_JS)
// %pointer_class(HLINCLIENT,HLINCLIENT_JS)

%include <carrays.i>

%array_class(unsigned long, LongArray);
%array_class(unsigned char, CharArray);
// %array_class(HLINHW,HLINHW_JS)

%include <linlib.h>




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
