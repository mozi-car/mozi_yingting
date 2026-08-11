%module xmlpp

%header %{
#include <windows.h>
#include <stdlib.h>
#include "zlgcan.h"
%}


%wrapper %{

%}

%include <stdint.i>
%include <windows.i>



%include <cpointer.i>

// %pointer_class(unsigned long,JSUINT64)
// %pointer_class(UINT64,TimeStamp)
// %pointer_class(long,JSINT64)
// %pointer_class(unsigned int,JSUINT32)
%pointer_class(DEVICE_HANDLE,DEVICE_HANDLE)
%pointer_class(CHANNEL_HANDLE,CHANNEL_HANDLE)


%include <carrays.i>

%array_class(uint32_t, U32Array);
%array_class(BYTE, ByteArray);
%array_class(ZCAN_Receive_Data, ReceiveDataArray);
%array_class(ZCAN_ReceiveFD_Data, ReceiveFDDataArray);

%include <canframe.h>
%include <zlgcan.h>








%inline %{
void LoadDll(const char* path) {
  SetDllDirectory(path);
}
void handleConver(CHANNEL_HANDLE channel,uint32_t* q){
  //q is array[2],CHANNEL_HANDLE is void* 64bit
  q[0] = (uint32_t)channel;
  q[1] = (uint32_t)(((uint64_t)channel)>>32);
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
