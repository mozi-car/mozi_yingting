/*
 * To include generic versions of the in typemaps, add:
 *
 * %typemap(in)        (void *, size_t) = (const void* buffer_data, const size_t buffer_len);
 * %typemap(typecheck) (void *, size_t) = (const void* buffer_data, const size_t buffer_len);
 *
 * or to discriminate by argument names:
 * %typemap(in)        (void *data, size_t length) = (const void* buffer_data, const size_t buffer_len);
 * %typemap(typecheck) (void *data, size_t length) = (const void* buffer_data, const size_t buffer_len);
 */

%typemap(in) (const size_t buffer_len,const void* buffer_data) {
  if ($input.IsBuffer()) {
    Napi::Buffer<char> buf = $input.As<Napi::Buffer<char>>();
    $2 = reinterpret_cast<void *>(buf.Data());
    $1 = buf.ByteLength();
    
  } else {
    SWIG_exception_fail(SWIG_TypeError, "in method '$symname', argument is not a Buffer");
  }
}

%typemap(typecheck, precedence=SWIG_TYPECHECK_VOIDPTR) (const size_t buffer_len,const void* buffer_data) {
  $2 = $input.IsBuffer();
}
