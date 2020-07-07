#include <mutex>
#include <memory>
#include <libplatform/libplatform.h>
#include <v8.h>

// The main interface that gets passed across the FFI that corresponds to a
// single `mini_v8::MiniV8` instance.
class Interface {
public:
  v8::Isolate* isolate;
  v8::ArrayBuffer::Allocator* allocator;
  v8::Persistent<v8::Context>* context;

  // Opens a new handle scope.
  //
  // TODO: Is this even used? Should it ever be used?
  template <typename F>
  auto scope(F&& func) const {
    const v8::Isolate::Scope isolate_scope(this->isolate);
    const v8::HandleScope handle_scope(this->isolate);
    return func(this->isolate);
  }

  // Opens a new handle scope and enters the context.
  template <typename F>
  auto context_scope(F&& func) const {
    const v8::Isolate::Scope isolate_scope(this->isolate);
    const v8::HandleScope handle_scope(this->isolate);
    const auto context = this->context->Get(this->isolate);
    const v8::Context::Scope context_scope(context);
    return func(this->isolate, context);
  }

  // Opens a new handle scope, enters the context, and opens a try-catch scope.
  template <typename F>
  auto try_catch(F&& func) const {
    const v8::Isolate::Scope isolate_scope(this->isolate);
    const v8::HandleScope handle_scope(this->isolate);
    const auto context = this->context->Get(this->isolate);
    const v8::Context::Scope context_scope(context);
    const v8::TryCatch try_catch(this->isolate);
    return func(this->isolate, context, &try_catch);
  }
};

// The type of value being passed.
enum ValueDescTag {
  Null,
  Undefined,
  Number,
  Boolean,
  Array,
  Function,
  Date,
  Object,
  String
};

// The value's payload.
union ValueDescPayload {
  uint8_t byte;
  double number;
  v8::Persistent<v8::Value>* value_ptr;
};

// An interface for passing values across the FFI between `v8::Local<v8::Value>`
// and `mini_v8::Value`.
struct ValueDesc {
  ValueDescPayload payload;
  uint8_t tag;
};

// An interface for passing possible exceptions across the FFI.
struct TryCatchDesc {
  ValueDesc value_desc;
  uint8_t is_exception;
};

// An interface for passing UTF-8 strings across the FFI.
struct Utf8Value {
  const uint8_t* data;
  int32_t length;
  const v8::String::Utf8Value* src;
};

// Initializes the V8 environment. Must be called before creating a V8 isolate.
// Can be called multiple times.
static void init_v8() {
  static std::unique_ptr<v8::Platform> current_platform = NULL;
  static std::mutex platform_lock;

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

// Converts a `v8::Local<v8::Value>` to a `ValueDesc`. Must be called while an
// isolate and a context are entered.
static ValueDesc value_to_desc(
  v8::Isolate* const isolate,
  const v8::Local<v8::Context> context,
  const v8::Local<v8::Value> value
) {
  ValueDesc desc { .payload = { .byte = 0 }, .tag = ValueDescTag::Undefined };

  if (value->IsUndefined()) {
    return desc;
  } else if (value->IsNull()) {
    desc.tag = ValueDescTag::Null;
    return desc;
  } else if (value->IsTrue()) {
    desc.tag = ValueDescTag::Boolean;
    desc.payload.byte = 1;
    return desc;
  } else if (value->IsFalse()) {
    desc.tag = ValueDescTag::Boolean;
    return desc;
  } else if (value->IsInt32()) {
    desc.tag = ValueDescTag::Number;
    desc.payload.number = (double)value->Int32Value(context).ToChecked();
    return desc;
  } else if (value->IsNumber()) {
    desc.tag = ValueDescTag::Number;
    desc.payload.number = value->NumberValue(context).ToChecked();
    return desc;
  } else if (value->IsDate()) {
    desc.tag = ValueDescTag::Date;
    desc.payload.number = v8::Local<v8::Date>::Cast(value)->ValueOf();
    return desc;
  } else if (value->IsString()) {
    desc.tag = ValueDescTag::String;
  } else if (value->IsArray()) {
    desc.tag = ValueDescTag::Array;
  } else if (value->IsFunction()) {
    desc.tag = ValueDescTag::Function;
  } else if (value->IsObject()) {
    desc.tag = ValueDescTag::Object;
  } else {
    return desc;
  }

  desc.payload.value_ptr = new v8::Persistent<v8::Value>(isolate, value);
  return desc;
}

// Converts a `ValueDesc` to a `v8::Local<v8::Value>`. Must be called while an
// isolate and a context are entered.
//
// This function frees the `ValueDesc`'s inner `v8::Persistent<v8::Value>`, if
// there is one. To avoid data leaks, functions that consume `ValueDesc`s should
// call this before there is any chance of exiting early.
static v8::Local<v8::Value> desc_to_value(
  v8::Isolate* const isolate,
  const v8::Local<v8::Context> context,
  const ValueDesc desc
) {
  v8::EscapableHandleScope scope(isolate);

  switch (desc.tag) {
    case ValueDescTag::Null:
      return scope.Escape(v8::Null(isolate));
    case ValueDescTag::Number:
      return scope.Escape(v8::Number::New(isolate, desc.payload.number));
    case ValueDescTag::Boolean:
      return scope.Escape(
        desc.payload.byte != 0 ? v8::True(isolate) : v8::False(isolate)
      );
    case ValueDescTag::Date:
      return scope.Escape(
        v8::Date::New(context, desc.payload.number).ToLocalChecked()
      );
    case ValueDescTag::Array:
    case ValueDescTag::Function:
    case ValueDescTag::Object:
    case ValueDescTag::String: {
      auto value_ptr = desc.payload.value_ptr;
      auto local = v8::Local<v8::Value>::New(isolate, *value_ptr);
      value_ptr->Reset();
      delete value_ptr;
      // TODO: Is this right? Does the final persistent handle being deleted
      // nullify the local handle we just created?
      return scope.Escape(local);
    }
    default:
      return scope.Escape(v8::Undefined(isolate));
  }
}

// Returns an error `TryCatchDesc` with the `v8::TryCatch`'s exception.
static TryCatchDesc try_catch_err(
  v8::Isolate* const isolate,
  const v8::Local<v8::Context> context,
  const v8::TryCatch* const try_catch
) {
  return {
    .value_desc = value_to_desc(isolate, context, try_catch->Exception()),
    .is_exception = 1
  };
}

// Returns an OK `TryCatchDesc` with the given value.
static TryCatchDesc try_catch_ok(
  v8::Isolate* const isolate,
  const v8::Local<v8::Context> context,
  const v8::Local<v8::Value> value
) {
  return {
    .value_desc = value_to_desc(isolate, context, value),
    .is_exception = 0
  };
}

// Returns an OK `TryCatchDesc` with no value attached.
static TryCatchDesc try_catch_ok_noval() {
  return {
    .value_desc = { .payload = { .byte = 0 }, .tag = ValueDescTag::Undefined },
    .is_exception = 0
  };
}

// Returns an OK `TryCatchDesc` with the raw `ValueDesc` attached.
static TryCatchDesc try_catch_ok_val(const ValueDesc desc) {
  return { .value_desc = desc, .is_exception = 0 };
}

// Creates a new `Interface`.
extern "C"
const Interface* mv8_interface_new() {
  init_v8();

  const auto interface = new Interface;

  interface->allocator = v8::ArrayBuffer::Allocator::NewDefaultAllocator();
  v8::Isolate::CreateParams create_params;
  create_params.array_buffer_allocator = interface->allocator;
  interface->isolate = v8::Isolate::New(create_params);

  const v8::Isolate::Scope isolate_scope(interface->isolate);
  const v8::HandleScope handle_scope(interface->isolate);

  const auto local_context = v8::Context::New(interface->isolate);
  interface->context = new v8::Persistent<v8::Context>();
  interface->context->Reset(interface->isolate, local_context);

  return interface;
}

// Drops an `Interface`, disposing its isolate.
extern "C"
void mv8_interface_drop(const Interface* const interface) {
  interface->context->Reset();
  delete interface->context;
  interface->isolate->Dispose();
  delete interface->allocator;
  delete interface;
}

/// Returns the interface's context's global object.
extern "C"
const v8::Persistent<v8::Value>* mv8_interface_global(
  const Interface* const interface
) {
  return interface->context_scope([](auto isolate, auto context) {
      return new v8::Persistent<v8::Value>(isolate, context->Global());
  });
}

// Evaluates a chunk of JavaScript.
extern "C"
TryCatchDesc mv8_interface_eval(
  const Interface* const interface,
  const char* const data,
  const size_t length
) {
  return interface->try_catch([=](auto isolate, auto context, auto try_catch) {
    const auto source = v8::String::NewFromUtf8(
      isolate,
      data,
      v8::NewStringType::kNormal,
      length
    ).ToLocalChecked();

    auto script = v8::Script::Compile(context, source);
    if (!script.IsEmpty()) {
      auto maybe_value = script.ToLocalChecked()->Run(context);
      if (!maybe_value.IsEmpty()) {
        return try_catch_ok(isolate, context, maybe_value.ToLocalChecked());
      }
    }

    return try_catch_err(isolate, context, try_catch);
  });
}

/// Sets user data at the given slot on the interface's isolate.
extern "C"
void mv8_interface_set_data(
  const Interface* const interface,
  const uint32_t slot,
  void* const data
) {
  interface->isolate->SetData(slot, data);
}

/// Gets the user data at the given slot on the interface's isolate.
extern "C"
const void* mv8_interface_get_data(
  const Interface* const interface,
  const uint32_t slot
) {
  return interface->isolate->GetData(slot);
}

// Creates a new reference to a value pointer.
extern "C"
const v8::Persistent<v8::Value>* mv8_value_ptr_clone(
  const Interface* const interface,
  const v8::Persistent<v8::Value>* const value_ptr
) {
  return new v8::Persistent<v8::Value>(interface->isolate, *value_ptr);
}

// Destroys a reference to a value pointer.
extern "C"
void mv8_value_ptr_drop(v8::Persistent<v8::Value>* const value_ptr) {
  value_ptr->Reset();
  delete value_ptr;
}

// Creates a new string from raw bytes.
extern "C"
const v8::Persistent<v8::Value>* mv8_string_new(
  const Interface* const interface,
  const char* const data,
  const size_t length
) {
  return interface->context_scope([=](auto isolate, auto) {
    const auto string = v8::String::NewFromUtf8(
      isolate,
      data,
      v8::NewStringType::kNormal,
      static_cast<int>(length)
    ).ToLocalChecked();
    return new v8::Persistent<v8::Value>(isolate, string);
  });
}

// Creates a new string from raw bytes.
extern "C"
Utf8Value mv8_string_to_utf8_value(
  const Interface* const interface,
  const v8::Persistent<v8::Value>* const value
) {
  return interface->context_scope([=](auto isolate, auto) {
    Utf8Value result;
    result.src = new v8::String::Utf8Value(isolate, value->Get(isolate));
    result.data = reinterpret_cast<const uint8_t*>(**result.src);
    result.length = result.src->length();
    return result;
  });
}

// Destroys a `Utf8Value`.
extern "C"
void mv8_utf8_value_drop(const Utf8Value value) {
  delete value.src;
}

// Creates a new, empty array.
extern "C"
const v8::Persistent<v8::Value>* mv8_array_new(
  const Interface* const interface
) {
  return interface->context_scope([=](auto isolate, auto) {
    return new v8::Persistent<v8::Value>(isolate, v8::Array::New(isolate, 0));
  });
}

// Returns the length of the given array.
extern "C"
uint32_t mv8_array_len(
  const Interface* const interface,
  const v8::Persistent<v8::Value>* const array
) {
  return interface->context_scope([=](auto isolate, auto) {
    return v8::Local<v8::Array>::Cast(array->Get(isolate))->Length();
  });
}

// Creates a new object.
extern "C"
const v8::Persistent<v8::Value>* mv8_object_new(
  const Interface* const interface
) {
  return interface->context_scope([=](auto isolate, auto) {
    return new v8::Persistent<v8::Value>(isolate, v8::Object::New(isolate));
  });
}

// Fetches an object's value by key.
extern "C"
TryCatchDesc mv8_object_get(
  const Interface* const interface,
  const v8::Persistent<v8::Value>* const object,
  const ValueDesc key_desc
) {
  return interface->try_catch([=](auto isolate, auto context, auto try_catch) {
    const auto local_object = v8::Local<v8::Object>::Cast(object->Get(isolate));
    const auto key = desc_to_value(isolate, context, key_desc);
    auto maybe_value = local_object->Get(context, key);
    if (maybe_value.IsEmpty()) {
      return try_catch_err(isolate, context, try_catch);
    }
    return try_catch_ok(isolate, context, maybe_value.ToLocalChecked());
  });
}

// Sets an object's property.
extern "C"
TryCatchDesc mv8_object_set(
  const Interface* const interface,
  const v8::Persistent<v8::Value>* const object,
  const ValueDesc key_desc,
  const ValueDesc value_desc
) {
  return interface->try_catch([=](auto isolate, auto context, auto try_catch) {
    const auto local_object = v8::Local<v8::Object>::Cast(object->Get(isolate));
    const auto key = desc_to_value(isolate, context, key_desc);
    const auto value = desc_to_value(isolate, context, value_desc);
    local_object->Set(context, key, value);
    if (try_catch->HasCaught()) {
      return try_catch_err(isolate, context, try_catch);
    }
    return try_catch_ok_noval();
  });
}

// Deletes an object's property.
extern "C"
TryCatchDesc mv8_object_remove(
  const Interface* const interface,
  const v8::Persistent<v8::Value>* const object,
  const ValueDesc key_desc
) {
  return interface->try_catch([=](auto isolate, auto context, auto try_catch) {
    const auto local_object = v8::Local<v8::Object>::Cast(object->Get(isolate));
    const auto key = desc_to_value(isolate, context, key_desc);
    local_object->Delete(context, key);
    if (try_catch->HasCaught()) {
      return try_catch_err(isolate, context, try_catch);
    }
    return try_catch_ok_noval();
  });
}

// Returns whether or not an object has a property with the given key.
extern "C"
TryCatchDesc mv8_object_has(
  const Interface* const interface,
  const v8::Persistent<v8::Value>* const object,
  const ValueDesc key_desc
) {
  return interface->try_catch([=](auto isolate, auto context, auto try_catch) {
    const auto local_object = v8::Local<v8::Object>::Cast(object->Get(isolate));
    const auto key = desc_to_value(isolate, context, key_desc);
    auto has = local_object->Has(context, key);
    if (try_catch->HasCaught()) {
      return try_catch_err(isolate, context, try_catch);
    }
    return try_catch_ok_val({
      .payload = { .byte = static_cast<uint8_t>(has.ToChecked() ? 1 : 0) },
      .tag = ValueDescTag::Boolean
    });
  });
}

// Coerces the given value into a boolean.
extern "C"
uint8_t mv8_coerce_boolean(
  const Interface* const interface,
  const ValueDesc desc
) {
  return interface->context_scope([=](auto isolate, auto context) {
    const auto value = desc_to_value(isolate, context, desc);
    return static_cast<uint8_t>(value->BooleanValue(isolate) ? 1 : 0);
  });
}

// Coerces the given value into a number.
extern "C"
TryCatchDesc mv8_coerce_number(
  const Interface* const interface,
  const ValueDesc desc
) {
  return interface->try_catch([=](auto isolate, auto context, auto try_catch) {
    const auto value = desc_to_value(isolate, context, desc);
    auto maybe_number = value->ToNumber(context);
    if (try_catch->HasCaught()) {
      return try_catch_err(isolate, context, try_catch);
    }
    return try_catch_ok_val({
      .payload = { .number = maybe_number.ToLocalChecked()->Value() },
      .tag = ValueDescTag::Number
    });
  });
}

// Coerces the given value into a string.
extern "C"
TryCatchDesc mv8_coerce_string(
  const Interface* const interface,
  const ValueDesc desc
) {
  return interface->try_catch([=](auto isolate, auto context, auto try_catch) {
    const auto value = desc_to_value(isolate, context, desc);
    auto maybe_string = value->ToString(context);
    if (try_catch->HasCaught()) {
      return try_catch_err(isolate, context, try_catch);
    }
    const auto string = maybe_string.ToLocalChecked();
    ValueDesc result;
    result.payload.value_ptr = new v8::Persistent<v8::Value>(isolate, string);
    result.tag = ValueDescTag::String;
    return try_catch_ok_val(result);
  });
}
