// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

///|
priv type NativeOperator

///|
priv type NativeResult

///|
#borrow(scheme)
extern "C" fn native_operator_new(scheme : Bytes) -> NativeResult = "moonbit_opendal_operator_new"

///|
#borrow(operator)
extern "C" fn native_operator_close(operator : NativeOperator) -> Unit = "moonbit_opendal_operator_close"

///|
#borrow(result)
extern "C" fn native_result_status(result : NativeResult) -> Int = "moonbit_opendal_result_status"

///|
#borrow(result)
extern "C" fn native_result_take_error_message(result : NativeResult) -> Bytes = "moonbit_opendal_result_take_error_message"

///|
#borrow(result)
extern "C" fn native_result_take_operator(
  result : NativeResult,
) -> NativeOperator = "moonbit_opendal_result_take_operator"

///|
test "native bridge owns an operator handle" {
  let result = native_operator_new(b"memory")
  assert_eq(native_result_status(result), 0)
  let operator = native_result_take_operator(result)
  native_operator_close(operator)
  native_operator_close(operator)
}

///|
test "native bridge preserves error messages" {
  let result = native_operator_new(b"unsupported")
  assert_eq(native_result_status(result), 1)
  assert_eq(
    native_result_take_error_message(result),
    b"only the memory service is enabled",
  )
}

///|
fn create_unclosed_operator() -> Unit {
  let result = native_operator_new(b"memory")
  ignore(native_result_take_operator(result))
}

///|
fn leave_result_unconsumed() -> Unit {
  ignore(native_operator_new(b"unsupported"))
}

///|
test "native bridge supports automatic cleanup" {
  create_unclosed_operator()
  leave_result_unconsumed()
}
