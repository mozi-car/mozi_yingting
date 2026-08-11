%module xmlpp

%header %{
#include <windows.h>
#include <stdlib.h>
#include "usb2lin_ex.h"
#include "usb_device.h"
%}


%wrapper %{

%}

%include <stdint.i>
%include <windows.i>



%include <cpointer.i>

// %pointer_class(unsigned long,JSUINT64)
// %pointer_class(UINT64,TimeStamp)
%pointer_class(unsigned char,UINT8P)
// %pointer_class(unsigned int,JSUINT32)



%include <carrays.i>

%array_class(LIN_EX_MSG, LIN_MSG_ARRAY);
%array_class(int, I32Array);
%array_class(unsigned int, U32Array);
%array_class(unsigned char,ByteArray);


%ignore LIN_EX_SetBusState;
%include <usb2lin_ex.h>

%include <usb_device.h>







%inline %{
void LoadDll(const char* path) {
  SetDllDirectory(path);
}

%}



%init %{

extern void CreateTSFN(const Napi::CallbackInfo &info);
extern void FreeTSFN(const Napi::CallbackInfo &info);
extern void SendLinMsg(const Napi::CallbackInfo &info);


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

do{
  Napi::PropertyDescriptor pd = Napi::PropertyDescriptor::Function("SendLinMsg", SendLinMsg);
  NAPI_CHECK_MAYBE(exports.DefineProperties({
	pd
  }));
} while (0);

%}