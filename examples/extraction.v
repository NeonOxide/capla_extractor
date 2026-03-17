From CertiCoq.Plugin Require Import CertiCoq.
From CertiCoq.Plugin Require Import Loader.
From Coq Require Import Uint63 .
From Coq Require Import PrimFloat .

(* Modular exponentiation *)

Axiom modular_exp : forall(base exp modulus: PrimInt63.int), PrimInt63.int.
Definition res := modular_exp 123 456 789.
CertiCoq Register [
  modular_exp  => "certirocq_modular_exp"
] Include ["modular_exp.h"].

CertiCoq Compile -O 1 -ext "_my_mod_exp" res.


(* Float increase *)
Axiom float_incr : forall(fl: PrimFloat.float), PrimFloat.float.

Definition float_res: PrimFloat.float := float_incr(2.0).

CertiCoq Register [
  float_incr  => "certirocq_float_incr" with tinfo
] Include ["float_incr.h"].

CertiCoq Compile -O 1 float_res.
