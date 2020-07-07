#include <mutex>
#include <memory>
#include <math.h>
#include <libplatform/libplatform.h>
#include <v8.h>
#include <iostream>

static std::unique_ptr<v8::Platform> current_platform = NULL;
static std::mutex platform_lock;

typedef struct {
  v8::Isolate* isolate;
  v8::ArrayBuffer::Allocator* allocator;
  v8::Persistent<v8::Context>* context;
  // Private symbol for storing a `RustCallback` pointer in a `v8::Function`:
  v8::Persistent<v8::Private>* priv_rust_callback;
} Context;

enum ValueTag {
  Null = 0,
  Undefined = 1,
  Number = 2,
  Boolean = 3,
  Array = 4,
  Function = 5,
  Date = 6,
  Object = 7,
  String = 8
};

typedef struct {
  uint8_t tag;
  union {
    struct { uint8_t e; };
    struct { double f; };
    struct { uint8_t b; };
    struct { v8::Persistent<v8::Value>* v; };
  };
} Value;

bool has_persistent_value(const Value* const value) {
  switch (value->tag) {
    case ValueTag::Array:
    case ValueTag::Function:
    case ValueTag::Object:
    case ValueTag::String:
      return true;

    default:
      return false;
  }
}

typedef struct {
  uint8_t exception;
  Value value;
} EvalResult;

typedef struct {
  const uint8_t* data;
  int32_t length;
  const v8::String::Utf8Value* src;
} Utf8Value;

static void init_v8() {
  if (current_platform != NULL) {
    return;
  }

  platform_lock.lock();

  if (current_platform == NULL) {
    v8::V8::InitializeICU();
    current_platform = v8::platform::NewDefaultPlatform();
    v8::V8::InitializePlatform(current_platform.get());
    v8::V8::Initialize();
  }

  platform_lock.unlock();
}

static Value to_ffi(Context* const context, const v8::Local<v8::Value> value) {
  const v8::Isolate::Scope isolate_scope(context->isolate);
  const v8::HandleScope scope(context->isolate);

  {
    auto typeof = value->TypeOf(context->isolate);
    auto utf8 = new v8::String::Utf8Value(context->isolate, typeof);
    const char* u = **utf8;
    // std::cout << "TYPEOF: " << u << std::endl;
  }

  Value out;

  if (value->IsNull()) {
    out.tag = ValueTag::Null;
    out.b = 0;
    return out;
  }

  if (value->IsUndefined()) {
    out.tag = ValueTag::Undefined;
    out.b = 0;
    return out;
  }

  if (value->IsTrue()) {
    out.tag = ValueTag::Boolean;
    out.b = 1;
    return out;
  }

  if (value->IsFalse()) {
    out.tag = ValueTag::Boolean;
    out.b = 0;
    return out;
  }

  const auto local_context = context->context->Get(context->isolate);
  const v8::Context::Scope context_scope(local_context);


  if (value->IsInt32()) {
    out.tag = ValueTag::Number;
    out.f = (double)value->Int32Value(local_context).ToChecked();
    // std::cout << "INT32: " << out.f << std::endl;
    return out;
  }

  if (value->IsNumber()) {
    out.tag = ValueTag::Number;
    out.f = value->NumberValue(local_context).ToChecked();
    // std::cout << "NUM: " << out.f << std::endl;
    return out;
  }

  if (value->IsDate()) {
    out.tag = ValueTag::Date;
    out.f = v8::Local<v8::Date>::Cast(value)->ValueOf();
    return out;
  }

  if (value->IsString()) {
    out.tag = ValueTag::String;
  } else if (value->IsArray()) {
    out.tag = ValueTag::Array;
  } else if (value->IsFunction()) {
    out.tag = ValueTag::Function;
  } else if (value->IsObject()) {
    out.tag = ValueTag::Object;
  } else {
    out.tag = ValueTag::Undefined;
    out.b = 0;
    return out;
  }

  auto persistent = new v8::Persistent<v8::Value>();
  persistent->Reset(context->isolate, value);
  out.v = persistent;
  return out;
}

static v8::Local<v8::Value> from_ffi(
  v8::Isolate* const isolate,
  const v8::Local<v8::Context> context,
  const Value* const ffi_val
) {
  const v8::Isolate::Scope isolate_scope(isolate);
  v8::EscapableHandleScope scope(isolate);
  const v8::Context::Scope context_scope(context);

  switch (ffi_val->tag) {
    case ValueTag::Null:
      return scope.Escape(v8::Null(isolate));
    case ValueTag::Number:
      return scope.Escape(v8::Number::New(isolate, ffi_val->f));
    case ValueTag::Boolean:
      return scope.Escape(ffi_val->b != 0 ?
        v8::True(isolate) :
        v8::False(isolate));
    case ValueTag::Date:
      return scope.Escape(v8::Date::New(context, ffi_val->f).ToLocalChecked());
    case ValueTag::Array:
    case ValueTag::Function:
    case ValueTag::Object:
    case ValueTag::String:
      return scope.Escape(v8::Local<v8::Value>::New(isolate, *ffi_val->v));
  }

  return scope.Escape(v8::Undefined(isolate));
}

typedef EvalResult (*rust_callback_wrapper)(
  const Context* const context,
  const void* const callback,
  const Value jsthis,
  const Value* const args,
  const int32_t num_args
);

typedef void (*rust_callback_drop)(void* callback);

static rust_callback_wrapper main_callback_wrapper_func = NULL;
static rust_callback_drop main_callback_drop_func = NULL;

typedef struct {
  void* callback;
  Context* context;
  v8::Persistent<v8::Value>* persistent;
} RustCallback;

extern "C" void mv8_value_drop(v8::Persistent<v8::Value>* const value);

static void rust_callback(const v8::FunctionCallbackInfo<v8::Value>& args) {
  const v8::Isolate::Scope isolate_scope(args.GetIsolate());
  const v8::HandleScope scope(args.GetIsolate());
  const auto ext = v8::Local<v8::External>::Cast(args.Data());

  const auto rcall = (RustCallback*)ext->Value();
  const auto local_context = rcall->context->context->Get(args.GetIsolate());
  const v8::Context::Scope context_scope(local_context);

  const auto length = args.Length();

  const auto recv = to_ffi(rcall->context, args.This().As<v8::Value>());
  // std::cout << "TAG: " << (uint64_t)recv.tag << std::endl;
  // std::cout << "VALUE: " << (uint64_t)recv.f << std::endl;
  const auto fargs = new Value[length];
  for (size_t i = 0; i < (size_t)length; i++) {
    fargs[i] = to_ffi(rcall->context, args[i].As<v8::Value>());
  }

  const EvalResult result = main_callback_wrapper_func(
    rcall->context,
    rcall->callback,
    recv,
    fargs,
    length
  );

  delete[] fargs;

  const auto value = from_ffi(args.GetIsolate(), local_context, &result.value);

  if (has_persistent_value(&result.value)) {
    mv8_value_drop(result.value.v);
  }

  if (result.exception != 0) {
    args.GetIsolate()->ThrowException(value);
  } else {
    args.GetReturnValue().Set(value);
  }
}

static void callback_drop_inner(
  v8::Isolate* const isolate,
  const RustCallback* const rcall
) {
  rcall->persistent->ClearWeak();
  main_callback_drop_func(rcall->callback);
  rcall->persistent->Reset();
  delete rcall->persistent;
  delete rcall;
  isolate->AdjustAmountOfExternalAllocatedMemory(-sizeof(RustCallback));
}

static void callback_drop(const v8::WeakCallbackInfo<RustCallback>& data) {
  callback_drop_inner(data.GetIsolate(), data.GetParameter());
}

constexpr uint16_t RUST_CALLBACK_CLASS_ID = 1001;

class PHV : public v8::PersistentHandleVisitor {
public:
  Context* context;

  PHV(Context* const context) : context(context) {}
  virtual ~PHV() {}

  virtual void VisitPersistentHandle(
    v8::Persistent<v8::Value>* const value,
    const uint16_t class_id
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_context = context->context->Get(context->isolate);
    const v8::Context::Scope context_scope(local_context);
    const auto priv_rust_callback = v8::Local<v8::Private>::New(
      context->isolate,
      *context->priv_rust_callback
    );
    if (class_id == RUST_CALLBACK_CLASS_ID) {
      const auto local_value = v8::Local<v8::Value>::New(
        context->isolate,
        *value
      );
      const auto object = v8::Local<v8::Object>::Cast(local_value);
      const auto ext = v8::Local<v8::External>::Cast(
        object->GetPrivate(local_context, priv_rust_callback).ToLocalChecked()
      );
      callback_drop_inner(context->isolate, (RustCallback*)ext->Value());
    }
  }
};

extern "C" {
  void mv8_init_set_callback_lifecycle_funcs(
    const rust_callback_wrapper wrapper_func,
    const rust_callback_drop drop_func
  ) {
    main_callback_wrapper_func = wrapper_func;
    main_callback_drop_func = drop_func;
  }

  Context* mv8_context_new() {
    init_v8();

    const auto context = new Context;

    context->allocator = v8::ArrayBuffer::Allocator::NewDefaultAllocator();
    v8::Isolate::CreateParams create_params;
    create_params.array_buffer_allocator = context->allocator;
    context->isolate = v8::Isolate::New(create_params);

    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope handle_scope(context->isolate);

    context->priv_rust_callback = new v8::Persistent<v8::Private>();
    context->priv_rust_callback->Reset(
      context->isolate,
      v8::Private::New(context->isolate)
    );

    const auto local_context = v8::Context::New(context->isolate);
    context->context = new v8::Persistent<v8::Context>();
    context->context->Reset(context->isolate, local_context);

    return context;
  }

  EvalResult mv8_context_eval(Context* context, const char* data, size_t length) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope handle_scope(context->isolate);
    const v8::TryCatch trycatch(context->isolate);
    const auto local_context = context->context->Get(context->isolate);
    const v8::Context::Scope context_scope(local_context);

    const auto source = v8::String::NewFromUtf8(
      context->isolate,
      data,
      v8::NewStringType::kNormal,
      length
    ).ToLocalChecked();

    auto script = v8::Script::Compile(local_context, source);

    EvalResult result;
    result.exception = 0;

    if (script.IsEmpty()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    auto maybe_val = script.ToLocalChecked()->Run(local_context);

    if (maybe_val.IsEmpty()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    result.value = to_ffi(context, maybe_val.ToLocalChecked());
    return result;
  }

  void mv8_context_drop(Context* context) {
    PHV phv(context);
    context->isolate->VisitHandlesWithClassIds(&phv);
    context->priv_rust_callback->Reset();
    delete context->priv_rust_callback;
    // Caution: `RustCallback`s are now invalidated, before the context itself
    // has been disposed. This is fine because we're assuming that execution has
    // completely halted in this context/isolate (we use one isolate per context
    // and are operating in a single-threaded environment).
    context->context->Reset();
    delete context->context;
    delete context->allocator;
    context->isolate->Dispose();
    delete context;
  }

  v8::Persistent<v8::Value>* mv8_context_global(Context* context) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    const v8::Context::Scope context_scope(local_context);
    return new v8::Persistent<v8::Value>(
      context->isolate,
      local_context->Global()
    );
  }

  void mv8_context_set_data(Context* context, uint32_t slot, void* data) {
    context->isolate->SetData(slot, data);
  }

  void* mv8_context_get_data(Context* context, uint32_t slot) {
    return context->isolate->GetData(slot);
  }

  v8::Persistent<v8::Value>* mv8_value_clone(
    Context* context,
    v8::Persistent<v8::Value>* value
  ) {
    return new v8::Persistent<v8::Value>(context->isolate, *value);
  }

  void mv8_value_drop(v8::Persistent<v8::Value>* const value) {
    value->Reset();
    delete value;
  }

  v8::Persistent<v8::Value>* mv8_string_create(
    Context* context,
    const char* data,
    size_t length
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    const v8::Context::Scope context_scope(local_context);
	  const auto string = v8::String::NewFromUtf8(
      context->isolate,
      data,
      v8::NewStringType::kNormal,
      (int)length
    ).ToLocalChecked();
    return new v8::Persistent<v8::Value>(context->isolate, string);
  }

  Utf8Value mv8_string_to_utf8_value(
    Context* context,
    v8::Persistent<v8::Value>* value
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    const v8::Context::Scope context_scope(local_context);
    const auto local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *value
    );

    Utf8Value result;
    result.src = new v8::String::Utf8Value(context->isolate, local_value);
    result.data = (const uint8_t*)**result.src;
    result.length = result.src->length();
    return result;
  }

  void mv8_utf8_value_drop(Utf8Value value) {
    delete value.src;
  }

  uint32_t mv8_array_length(
    Context* context,
    v8::Persistent<v8::Value>* array_val
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *array_val
    );
    const v8::Local<v8::Array> array = v8::Local<v8::Array>::Cast(local_value);
    return array->Length();
  }

  v8::Persistent<v8::Value>* mv8_object_create(Context* context) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    const v8::Context::Scope mv8_context_scope(local_context);
    const auto object = v8::Object::New(context->isolate);
    return new v8::Persistent<v8::Value>(context->isolate, object);
  }

  v8::Persistent<v8::Value>* mv8_array_create(Context* context) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    const v8::Context::Scope context_scope(local_context);
    const auto array = v8::Array::New(context->isolate, 0);
    return new v8::Persistent<v8::Value>(context->isolate, array);
  }

  EvalResult mv8_object_get(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    Value ffi_key
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);

    const auto local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto object = v8::Local<v8::Object>::Cast(local_value);
    const auto key = from_ffi(context->isolate, local_context, &ffi_key);

    const v8::TryCatch trycatch(context->isolate);

    auto maybe_val = object->Get(local_context, key);
    EvalResult result;

    if (trycatch.HasCaught()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    result.exception = 0;
    if (maybe_val.IsEmpty()) {
      result.value.tag = ValueTag::Undefined;
      return result;
    }

    result.value = to_ffi(context, maybe_val.ToLocalChecked());
    return result;
  }

  EvalResult mv8_object_set(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    Value ffi_key,
    Value ffi_value
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto object = v8::Local<v8::Object>::Cast(local_value);
    const auto key = from_ffi(context->isolate, local_context, &ffi_key);
    const auto value = from_ffi(context->isolate, local_context, &ffi_value);

    const v8::TryCatch trycatch(context->isolate);
    object->Set(local_context, key, value);

    EvalResult result;

    if (trycatch.HasCaught()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    result.exception = 0;
    return result;
  }

  Value mv8_object_get_index(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    uint32_t index
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto object = v8::Local<v8::Object>::Cast(local_value);
    auto maybe_val = object->Get(local_context, index);

    if (maybe_val.IsEmpty()) {
      Value out;
      out.tag = ValueTag::Undefined;
      return out;
    }

    return to_ffi(context, maybe_val.ToLocalChecked());
  }

  void mv8_object_set_index(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    uint32_t index,
    Value ffi_value
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto object = v8::Local<v8::Object>::Cast(local_value);
    const auto value = from_ffi(context->isolate, local_context, &ffi_value);

    object->Set(local_context, index, value);
  }

  EvalResult mv8_object_remove(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    Value ffi_key
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto object = v8::Local<v8::Object>::Cast(local_value);
    const auto key = from_ffi(context->isolate, local_context, &ffi_key);

    const v8::TryCatch trycatch(context->isolate);
    auto deleted = object->Delete(local_context, key);

    EvalResult result;

    if (trycatch.HasCaught()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    result.value.tag = ValueTag::Boolean;
    result.exception = 0;

    if (deleted.IsNothing()) {
      result.value.b = 0;
      return result;
    }

    result.value.b = deleted.ToChecked() ? 1 : 0;
    return result;
  }

  EvalResult mv8_object_contains_key(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    Value ffi_key
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto object = v8::Local<v8::Object>::Cast(local_value);
    const auto key = from_ffi(context->isolate, local_context, &ffi_key);

    const v8::TryCatch trycatch(context->isolate);
    auto has = object->Has(local_context, key);

    EvalResult result;

    if (trycatch.HasCaught()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    result.value.tag = ValueTag::Boolean;
    result.exception = 0;

    if (has.IsNothing()) {
      result.value.b = 0;
      return result;
    }

    result.value.b = has.ToChecked() ? 1 : 0;
    return result;
  }

  v8::Persistent<v8::Value>* mv8_object_keys(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    uint8_t include_inherited
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);
    const auto local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto object = v8::Local<v8::Object>::Cast(local_value);

    const v8::TryCatch trycatch(context->isolate);
    auto maybe_array = include_inherited != 0 ?
      object->GetPropertyNames(local_context) :
      object->GetOwnPropertyNames(local_context);
    const auto array = trycatch.HasCaught() || maybe_array.IsEmpty() ?
      v8::Array::New(context->isolate, 0) :
      maybe_array.ToLocalChecked();
    return new v8::Persistent<v8::Value>(context->isolate, array);
  }

  uint8_t mv8_coerce_boolean(
    Context* context,
    Value ffi_value
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto value = from_ffi(context->isolate, local_context, &ffi_value);
    return value->BooleanValue(context->isolate) ? 1 : 0;
  }

  EvalResult mv8_coerce_number(
    Context* context,
    Value ffi_value
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto value = from_ffi(context->isolate, local_context, &ffi_value);

    const v8::TryCatch trycatch(context->isolate);
    auto number = value->ToNumber(local_context);

    EvalResult result;

    if (trycatch.HasCaught()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    const auto number_val = !number.IsEmpty() ?
      number.ToLocalChecked()->Value() :
      NAN;

    result.value.tag = ValueTag::Number;
    result.value.f = number_val;
    result.exception = 0;
    return result;
  }

  EvalResult mv8_coerce_string(
    Context* context,
    Value ffi_value
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    const auto value = from_ffi(context->isolate, local_context, &ffi_value);
    const v8::TryCatch trycatch(context->isolate);
    auto string = value->ToString(local_context);

    EvalResult result;

    if (trycatch.HasCaught()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    const auto string_val = !string.IsEmpty() ?
      string.ToLocalChecked() :
      v8::String::Empty(context->isolate);

    result.value.tag = ValueTag::String;
    const auto persistent = new v8::Persistent<v8::Value>();
    persistent->Reset(context->isolate, string_val);
    result.value.v = persistent;
    result.exception = 0;
    return result;
  }

  EvalResult mv8_function_call(
    Context* context,
    v8::Persistent<v8::Value>* function_val,
    Value ffi_this,
    const Value* ffi_args,
    int32_t num_args
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    const v8::Context::Scope context_scope(local_context);

    const auto local_val = v8::Local<v8::Value>::New(
      context->isolate,
      *function_val
    );
    const auto function = v8::Local<v8::Function>::Cast(local_val);

    const auto local_this = from_ffi(
      context->isolate,
      local_context,
      &ffi_this
    );

    const auto local_args = new v8::Local<v8::Value>[num_args];
    for (size_t i = 0; i < (size_t)num_args; i++) {
      local_args[i] = from_ffi(context->isolate, local_context, &ffi_args[i]);
    }

    const v8::TryCatch trycatch(context->isolate);
    EvalResult result;
    result.exception = 0;

    auto maybe_val = function->Call(
      local_context,
      local_this,
      (int)num_args,
      local_args
    );

    delete[] local_args;

    if (maybe_val.IsEmpty()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    result.value = to_ffi(context, maybe_val.ToLocalChecked());
    return result;
  }

  v8::Persistent<v8::Value>* mv8_function_create(
    Context* context,
    void* callback
  ) {
    const v8::Isolate::Scope isolate_scope(context->isolate);
    const v8::HandleScope scope(context->isolate);

    const auto local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    const v8::Context::Scope context_scope(local_context);

    const auto rcall = new RustCallback;
    rcall->context = context;
    rcall->callback = callback;

    const auto ext = v8::External::New(context->isolate, rcall);

    const auto func_temp = v8::FunctionTemplate::New(
      context->isolate,
      rust_callback,
      ext
    );

    const auto func = func_temp->GetFunction(local_context).ToLocalChecked();

    const auto funcobj = v8::Local<v8::Object>::Cast(func);
    const auto priv_rust_callback = v8::Local<v8::Private>::New(
      context->isolate,
      *context->priv_rust_callback
    );
    funcobj->SetPrivate(local_context, priv_rust_callback, ext);

    const auto persistent = new v8::Persistent<v8::Value>(
      context->isolate,
      func
    );

    const auto weak_persistent = new v8::Persistent<v8::Value>(
      context->isolate,
      *persistent
    );
    rcall->persistent = weak_persistent;
    weak_persistent->SetWrapperClassId(RUST_CALLBACK_CLASS_ID);
    weak_persistent->SetWeak(
      rcall,
      callback_drop,
      v8::WeakCallbackType::kParameter
    );
    context->isolate->AdjustAmountOfExternalAllocatedMemory(
      sizeof(RustCallback)
    );

    return persistent;
  }
}
