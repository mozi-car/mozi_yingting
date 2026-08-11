%module xmlpp

%header %{
#include <windows.h>
#include "sa.h"
%}


%wrapper %{

%}





%include <cpointer.i>

%pointer_class(unsigned int,UINT32_PTR)

%include <carrays.i>

%array_class(unsigned char, UINT8_ARRAY);
%array_class(char, INT8_ARRAY);


%include <sa.h>

