/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

#include <limits.h>
#include <moonbit.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef struct opendal_moonbit_operator opendal_moonbit_operator_t;
typedef struct opendal_moonbit_result opendal_moonbit_result_t;

extern opendal_moonbit_result_t *
opendal_moonbit_operator_new(const uint8_t *scheme, uint32_t scheme_len);
extern void opendal_moonbit_operator_close(opendal_moonbit_operator_t *operator_);
extern void opendal_moonbit_operator_free(opendal_moonbit_operator_t *operator_);
extern int32_t
opendal_moonbit_result_status(const opendal_moonbit_result_t *result);
extern const uint8_t *opendal_moonbit_result_error_message_data(
    const opendal_moonbit_result_t *result);
extern size_t opendal_moonbit_result_error_message_len(
    const opendal_moonbit_result_t *result);
extern opendal_moonbit_operator_t *
opendal_moonbit_result_take_operator(opendal_moonbit_result_t *result);
extern void opendal_moonbit_result_free(opendal_moonbit_result_t *result);

typedef struct {
  opendal_moonbit_operator_t *inner;
} moonbit_opendal_operator_t;

typedef struct {
  opendal_moonbit_result_t *inner;
} moonbit_opendal_result_t;

static uint32_t moonbit_bytes_len(moonbit_bytes_t bytes) {
  return (uint32_t)Moonbit_array_length(bytes);
}

static moonbit_bytes_t copy_bytes(const uint8_t *data, size_t len) {
  if (len > INT32_MAX || (len != 0 && data == NULL)) {
    static const uint8_t fallback[] = "native result is unavailable";
    data = fallback;
    len = sizeof(fallback) - 1;
  }
  moonbit_bytes_t output = moonbit_make_bytes((int32_t)len, 0);
  if (len != 0) {
    memcpy(output, data, len);
  }
  return output;
}

static void operator_finalize(void *payload) {
  moonbit_opendal_operator_t *operator_ = payload;
  if (operator_->inner != NULL) {
    opendal_moonbit_operator_free(operator_->inner);
    operator_->inner = NULL;
  }
}

static moonbit_opendal_operator_t *
operator_external(opendal_moonbit_operator_t *inner) {
  moonbit_opendal_operator_t *operator_ = moonbit_make_external_object(
      operator_finalize, (uint32_t)sizeof(moonbit_opendal_operator_t));
  operator_->inner = inner;
  return operator_;
}

static void result_finalize(void *payload) {
  moonbit_opendal_result_t *result = payload;
  if (result->inner != NULL) {
    opendal_moonbit_result_free(result->inner);
    result->inner = NULL;
  }
}

static moonbit_opendal_result_t *
result_external(opendal_moonbit_result_t *inner) {
  moonbit_opendal_result_t *result = moonbit_make_external_object(
      result_finalize, (uint32_t)sizeof(moonbit_opendal_result_t));
  result->inner = inner;
  return result;
}

MOONBIT_FFI_EXPORT moonbit_opendal_result_t *
moonbit_opendal_operator_new(moonbit_bytes_t scheme) {
  return result_external(
      opendal_moonbit_operator_new(scheme, moonbit_bytes_len(scheme)));
}

MOONBIT_FFI_EXPORT void
moonbit_opendal_operator_close(moonbit_opendal_operator_t *operator_) {
  if (operator_ != NULL) {
    opendal_moonbit_operator_close(operator_->inner);
  }
}

MOONBIT_FFI_EXPORT int32_t
moonbit_opendal_result_status(moonbit_opendal_result_t *result) {
  return opendal_moonbit_result_status(result == NULL ? NULL : result->inner);
}

MOONBIT_FFI_EXPORT moonbit_bytes_t
moonbit_opendal_result_take_error_message(moonbit_opendal_result_t *result) {
  opendal_moonbit_result_t *inner = result == NULL ? NULL : result->inner;
  moonbit_bytes_t message = copy_bytes(
      opendal_moonbit_result_error_message_data(inner),
      opendal_moonbit_result_error_message_len(inner));
  opendal_moonbit_result_free(inner);
  if (result != NULL) {
    result->inner = NULL;
  }
  return message;
}

MOONBIT_FFI_EXPORT moonbit_opendal_operator_t *
moonbit_opendal_result_take_operator(moonbit_opendal_result_t *result) {
  opendal_moonbit_result_t *inner = result == NULL ? NULL : result->inner;
  opendal_moonbit_operator_t *operator_ =
      opendal_moonbit_result_take_operator(inner);
  opendal_moonbit_result_free(inner);
  if (result != NULL) {
    result->inner = NULL;
  }
  return operator_external(operator_);
}
