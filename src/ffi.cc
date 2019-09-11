#include <mutex>
#include <memory>
#include <math.h>
#include <libplatform/libplatform.h>
#include <v8.h>

static std::unique_ptr<v8::Platform> current_platform = NULL;
static std::mutex platform_lock;

typedef struct {
  v8::Isolate* isolate;
  v8::ArrayBuffer::Allocator* allocator;
  v8::Persistent<v8::Context>* context;
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
  unsigned int tag;
  union {
    struct { uint8_t e; };
    struct { double f; };
    struct { uint8_t b; };
    struct { v8::Persistent<v8::Value>* v; };
  };
} Value;

typedef struct {
  uint8_t exception;
  Value value;
} EvalResult;

typedef struct {
  const uint8_t* data;
  int32_t length;
  v8::String::Utf8Value* src;
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

static Value to_ffi(Context* context, v8::Local<v8::Value> value) {
  v8::Isolate::Scope isolate_scope(context->isolate);
  v8::HandleScope scope(context->isolate);

  Value out;

  if (value->IsNull()) {
    out.tag = ValueTag::Null;
    return out;
  }

  if (value->IsUndefined()) {
    out.tag = ValueTag::Undefined;
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

  v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
    context->isolate,
    *context->context
  );

  if (value->IsInt32()) {
    out.tag = ValueTag::Number;
    out.f = (double)value->Int32Value(local_context).ToChecked();
    return out;
  }

  if (value->IsNumber()) {
    out.tag = ValueTag::Number;
    out.f = value->NumberValue(local_context).ToChecked();
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
    return out;
  }

  v8::Persistent<v8::Value>* persistent = new v8::Persistent<v8::Value>();
  persistent->Reset(context->isolate, value);
  out.v = persistent;
  return out;
}

static v8::Local<v8::Value> from_ffi(
  v8::Isolate* isolate,
  v8::Local<v8::Context> context,
  Value ffi_value
) {
  v8::EscapableHandleScope scope(isolate);

  switch (ffi_value.tag) {
    case ValueTag::Null:
      return scope.Escape(v8::Null(isolate));
    case ValueTag::Number:
      return scope.Escape(v8::Number::New(isolate, ffi_value.f));
    case ValueTag::Boolean:
      return scope.Escape(ffi_value.b != 0 ?
        v8::True(isolate) :
        v8::False(isolate));
    case ValueTag::Date:
      return scope.Escape(v8::Date::New(context, ffi_value.f).ToLocalChecked());
    case ValueTag::Array:
    case ValueTag::Function:
    case ValueTag::Object:
    case ValueTag::String:
      v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
        isolate,
        *ffi_value.v
      );
      return scope.Escape(local_value);
  }

  return scope.Escape(v8::Undefined(isolate));
}

extern "C" {
  Context* context_new() {
    init_v8();

    Context* context = new Context;

    context->allocator = v8::ArrayBuffer::Allocator::NewDefaultAllocator();
    v8::Isolate::CreateParams create_params;
    create_params.array_buffer_allocator = context->allocator;
    context->isolate = v8::Isolate::New(create_params);

    {
      v8::Isolate::Scope isolate_scope(context->isolate);
      v8::HandleScope handle_scope(context->isolate);

      v8::Local<v8::Context> local_context = v8::Context::New(context->isolate);
      context->context = new v8::Persistent<v8::Context>();
      context->context->Reset(context->isolate, local_context);
    }

    return context;
  }

  EvalResult context_eval(Context* context, const char *data, size_t length) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope handle_scope(context->isolate);
    v8::TryCatch trycatch(context->isolate);
    v8::Local<v8::Context> local_context = context->context->Get(
      context->isolate
    );
    v8::Context::Scope context_scope(local_context);

    v8::Local<v8::String> source = v8::String::NewFromUtf8(
      context->isolate,
      data,
      v8::NewStringType::kNormal,
      length
    ).ToLocalChecked();

    v8::MaybeLocal<v8::Script> script = v8::Script::Compile(
      local_context,
      source
    );

    EvalResult result;
    result.exception = 0;

    if (script.IsEmpty()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    v8::MaybeLocal<v8::Value> maybe_val = script.ToLocalChecked()->Run(
      local_context
    );

    if (maybe_val.IsEmpty()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    v8::Local<v8::Value> value = maybe_val.ToLocalChecked();
    result.value = to_ffi(context, value);
    return result;
  }

  void context_drop(Context* context) {
    context->context->Reset();
    delete context->context;
    delete context->allocator;
    context->isolate->Dispose();
    delete context;
  }

  v8::Persistent<v8::Value>* value_clone(
    Context* context,
    v8::Persistent<v8::Value>* value
  ) {
    return new v8::Persistent<v8::Value>(context->isolate, *value);
  }

  void value_drop(v8::Persistent<v8::Value>* value) {
    value->Reset();
    delete value;
  }

  v8::Persistent<v8::Value>* string_create(
    Context* context,
    const char *data,
    size_t length
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    v8::Context::Scope context_scope(local_context);
	  v8::Local<v8::String> string = v8::String::NewFromUtf8(
      context->isolate,
      data,
      v8::NewStringType::kNormal,
      (int)length
    ).ToLocalChecked();
    return new v8::Persistent<v8::Value>(context->isolate, string);
  }

  Utf8Value string_to_utf8_value(
    Context* context,
    v8::Persistent<v8::Value>* value
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *value
    );

    Utf8Value result;
    result.src = new v8::String::Utf8Value(context->isolate, local_value);
    result.data = (const uint8_t*)**result.src;
    result.length = result.src->length();
    return result;
  }

  void utf8_value_drop(Utf8Value value) {
    delete value.src;
  }

  uint32_t array_length(
    Context* context,
    v8::Persistent<v8::Value>* array_val
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *array_val
    );

    v8::Local<v8::Array> array = v8::Local<v8::Array>::Cast(local_value);
    return array->Length();
  }

  v8::Persistent<v8::Value>* object_create(Context* context) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    v8::Context::Scope context_scope(local_context);

    v8::Local<v8::Object> object = v8::Object::New(context->isolate);
    return new v8::Persistent<v8::Value>(context->isolate, object);
  }

  v8::Persistent<v8::Value>* array_create(Context* context) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );
    v8::Context::Scope context_scope(local_context);

    v8::Local<v8::Array> array = v8::Array::New(context->isolate, 0);
    return new v8::Persistent<v8::Value>(context->isolate, array);
  }

  EvalResult object_get(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    Value ffi_key
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);

    v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Object> object = v8::Local<v8::Object>::Cast(local_value);
    v8::Local<v8::Value> key = from_ffi(
      context->isolate,
      local_context,
      ffi_key
    );

    v8::TryCatch trycatch(context->isolate);

    v8::MaybeLocal<v8::Value> maybe_val = object->Get(local_context, key);
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

  EvalResult object_set(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    Value ffi_key,
    Value ffi_value
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Object> object = v8::Local<v8::Object>::Cast(local_value);
    v8::Local<v8::Value> key = from_ffi(
      context->isolate,
      local_context,
      ffi_key
    );
    v8::Local<v8::Value> value = from_ffi(
      context->isolate,
      local_context,
      ffi_value
    );

    v8::TryCatch trycatch(context->isolate);
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

  Value object_get_index(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    uint32_t index
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Object> object = v8::Local<v8::Object>::Cast(local_value);
    v8::MaybeLocal<v8::Value> maybe_val = object->Get(local_context, index);

    if (maybe_val.IsEmpty()) {
      Value out;
      out.tag = ValueTag::Undefined;
      return out;
    }

    return to_ffi(context, maybe_val.ToLocalChecked());
  }

  void object_set_index(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    uint32_t index,
    Value ffi_value
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Object> object = v8::Local<v8::Object>::Cast(local_value);
    v8::Local<v8::Value> value = from_ffi(
      context->isolate,
      local_context,
      ffi_value
    );

    object->Set(local_context, index, value);
  }

  EvalResult object_remove(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    Value ffi_key
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Object> object = v8::Local<v8::Object>::Cast(local_value);
    v8::Local<v8::Value> key = from_ffi(
      context->isolate,
      local_context,
      ffi_key
    );

    v8::TryCatch trycatch(context->isolate);
    v8::Maybe<bool> deleted = object->Delete(local_context, key);

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

  EvalResult object_contains_key(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    Value ffi_key
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Object> object = v8::Local<v8::Object>::Cast(local_value);
    v8::Local<v8::Value> key = from_ffi(
      context->isolate,
      local_context,
      ffi_key
    );

    v8::TryCatch trycatch(context->isolate);
    v8::Maybe<bool> has = object->Has(local_context, key);

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

  v8::Persistent<v8::Value>* object_keys(
    Context* context,
    v8::Persistent<v8::Value>* object_val,
    uint8_t include_inherited
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);
    v8::Local<v8::Value> local_value = v8::Local<v8::Value>::New(
      context->isolate,
      *object_val
    );

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Object> object = v8::Local<v8::Object>::Cast(local_value);

    v8::TryCatch trycatch(context->isolate);
    v8::MaybeLocal<v8::Array> maybe_array;
    if (include_inherited != 0) {
      maybe_array = object->GetPropertyNames(local_context);
    } else {
      maybe_array = object->GetOwnPropertyNames(local_context);
    }

    v8::Local<v8::Array> array;
    if (trycatch.HasCaught() || maybe_array.IsEmpty()) {
      array = v8::Array::New(context->isolate, 0);
    } else {
      array = maybe_array.ToLocalChecked();
    }

    return new v8::Persistent<v8::Value>(context->isolate, array);
  }

  uint8_t coerce_boolean(
    Context* context,
    Value ffi_value
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Value> value = from_ffi(
      context->isolate,
      local_context,
      ffi_value
    );

    return value->BooleanValue(context->isolate) ? 1 : 0;
  }

  EvalResult coerce_number(
    Context* context,
    Value ffi_value
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Value> value = from_ffi(
      context->isolate,
      local_context,
      ffi_value
    );

    v8::TryCatch trycatch(context->isolate);

    v8::MaybeLocal<v8::Number> number = value->ToNumber(context->isolate);

    EvalResult result;

    if (trycatch.HasCaught()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    float number_val = NAN;

    if (!number.IsEmpty()) {
      number_val = number.ToLocalChecked()->Value();
    }

    result.value.tag = ValueTag::Number;
    result.value.f = number_val;
    result.exception = 0;
    return result;
  }

  EvalResult coerce_string(
    Context* context,
    Value ffi_value
  ) {
    v8::Isolate::Scope isolate_scope(context->isolate);
    v8::HandleScope scope(context->isolate);

    v8::Local<v8::Context> local_context = v8::Local<v8::Context>::New(
      context->isolate,
      *context->context
    );

    v8::Local<v8::Value> value = from_ffi(
      context->isolate,
      local_context,
      ffi_value
    );

    v8::TryCatch trycatch(context->isolate);

    v8::MaybeLocal<v8::String> string = value->ToString(context->isolate);

    EvalResult result;

    if (trycatch.HasCaught()) {
      result.value = to_ffi(context, trycatch.Exception());
      result.exception = 1;
      return result;
    }

    v8::Local<v8::String> string_val;

    if (!string.IsEmpty()) {
      string_val = string.ToLocalChecked();
    } else {
      string_val = v8::String::Empty(context->isolate);
    }

    result.value.tag = ValueTag::String;
    v8::Persistent<v8::Value>* persistent = new v8::Persistent<v8::Value>();
    persistent->Reset(context->isolate, string_val);
    result.value.v = persistent;
    result.exception = 0;
    return result;
  }
}
