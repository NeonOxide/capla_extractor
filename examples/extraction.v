From CertiRocq.Plugin Require Import CertiRocq.
From CertiRocq.Plugin Require Import Loader.
From Stdlib.Strings Require Import PrimString.
From Stdlib.Numbers.Cyclic.Int63 Require Import Uint63. 
From Stdlib Require Export PrimFloat.



(* Modular exponentiation *)

Axiom modular_exp : forall(base exp modulus: PrimInt63.int), PrimInt63.int.
Definition res := modular_exp 123 456 789.
CertiRocq Register [
  modular_exp  => "certirocq_modular_exp"
] Include ["modular_exp.h"].

CertiRocq Compile -O 1 -ext "_my_mod_exp" res.


(* Float increase *)
Axiom float_incr : forall(fl: PrimFloat.float), PrimFloat.float.

Definition float_res: PrimFloat.float := float_incr(2.0).

CertiRocq Register [
  float_incr  => "certirocq_float_incr" with tinfo
] Include ["float_incr.h"].

CertiRocq Compile -O 1 -ext "_my_float_incr" float_res.


(* Sha256 *)
From Stdlib.Strings Require Import PrimString.
Definition myPrimString: PrimString.string := "Hello World".
Axiom sha256 : (PrimString.string) -> (PrimString.string).
Definition sha_res := sha256 myPrimString.

CertiRocq Register [
  sha256  => "certirocq_sha256" with tinfo
] Include ["sha256.h"].

CertiRocq Compile -O 1 -ext "_my_sha256" sha_res.
