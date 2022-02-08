// MIT License
//
// Copyright (c) 2020-2021 Tobias Pfeiffer
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#![allow(non_snake_case, clippy::type_complexity, clippy::wrong_self_convention, non_camel_case_types)]

use {
	types::{*, Ordering, Error},
	crate::*,
	std::{marker::PhantomData, sync::atomic::AtomicUsize},
	serde::{*, de::DeserializeOwned}
};

#[cfg(feature = "async")]
use std::{future::Future, pin::Pin};

static QUERY_VAR_COUNTER: AtomicUsize = AtomicUsize::new(0);

mod types {
	pub struct Top;
	pub struct Datum;
	pub struct Null;
	pub struct Bool;
	pub struct Number;
	pub struct String;
	pub struct Object;
	pub struct SingleSelection;
	pub struct Array;
	pub struct Sequence;
	pub struct Stream;
	pub struct StreamSelection;
	pub struct Table;
	pub struct Database;
	pub struct Function<A, O>(super::PhantomData<dyn FnOnce(A) -> O>);
	pub struct Ordering;
	pub struct PathSpec;
	pub struct Error;
}

pub type ReqlTop             = Top;
pub type ReqlDatum           = Datum;
pub type ReqlNull            = Null;
pub type ReqlBool            = Bool;
pub type ReqlNumber          = Number;
pub type ReqlString          = String;
pub type ReqlObject          = Object;
pub type ReqlSingleSelection = SingleSelection;
pub type ReqlArray           = Array;
pub type ReqlSequence        = Sequence;
pub type ReqlStream          = Stream;
pub type ReqlStreamSelection = StreamSelection;
pub type ReqlTable           = Table;
pub type ReqlDatabase        = Database;
pub type ReqlFunction<A, O>  = Function<A, O>;
pub type ReqlOrdering        = Ordering;
pub type ReqlPathSpec        = PathSpec;
pub type ReqlError           = Error;

pub trait Term<T>: Serialize {}

pub trait Run: Serialize + Sized {
	/// Run a query on the given connection. Returns a cursor that yields elements of the type `T`.
	fn run<T: DeserializeOwned>(self, conn: &Connection, options: Option<QueryOptions>) -> Result<Cursor<T>> {
		conn.query(self, options)
	}
	
	/// Run a query on the given connection asynchronously. Returns a cursor that yields elements of the type `T`.
	#[cfg(feature = "async")]
	fn run_async<'a, T: DeserializeOwned + Send + Sync + Unpin + 'static>(
		self, conn: &'a AsyncConnection, options: Option<QueryOptions<'a>>
	) -> Pin<Box<dyn Future<Output = Result<AsyncCursor<T>>> + Send + 'a>> where Self: 'a + Send + Sync + Unpin {
		Box::pin(conn.query(self, options))
	}
	
	/// Run a query on the given connection. Returns an element of the type `T`.
	fn run_single<T: DeserializeOwned>(self, conn: &Connection, options: Option<QueryOptions>) -> Result<Option<T>> {
		conn.query(self, options)?.next().transpose()
	}
	
	/// Run a query on the given connection asynchronously. Returns an element of the type `T`.
	#[cfg(feature = "async")]
	fn run_single_async<'a, T: DeserializeOwned + Send + Sync + Unpin + 'static>(
		self, conn: &'a AsyncConnection, options: Option<QueryOptions<'a>>
	) -> Pin<Box<dyn Future<Output = Result<Option<T>>> + Send + 'a>> where Self: 'a + Send + Sync + Unpin {
		Box::pin(async move {
			smol::stream::StreamExt::next(&mut conn
				.query(self, options).await?).await.transpose()
		})
	}
}

impl<T: Serialize + Sized> Run for T {}

// TYPES IMPLS

macro_rules! impl_types {
	(impl $( ( $( $generics:tt )* ) )? Top for $ty:ty ) => {
		impl $( < $( $generics )* > )* Term<Top> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Datum for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>   for $ty {}
		impl $( < $( $generics )* > )* Term<Datum> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Null for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>   for $ty {}
		impl $( < $( $generics )* > )* Term<Datum> for $ty {}
		impl $( < $( $generics )* > )* Term<Null>  for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Bool for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>   for $ty {}
		impl $( < $( $generics )* > )* Term<Datum> for $ty {}
		impl $( < $( $generics )* > )* Term<Bool>  for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Number for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>    for $ty {}
		impl $( < $( $generics )* > )* Term<Datum>  for $ty {}
		impl $( < $( $generics )* > )* Term<Number> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? String for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>    for $ty {}
		impl $( < $( $generics )* > )* Term<Datum>  for $ty {}
		impl $( < $( $generics )* > )* Term<String> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Object for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>    for $ty {}
		impl $( < $( $generics )* > )* Term<Datum>  for $ty {}
		impl $( < $( $generics )* > )* Term<Object> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? SingleSelection for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>             for $ty {}
		impl $( < $( $generics )* > )* Term<Datum>           for $ty {}
		impl $( < $( $generics )* > )* Term<Object>          for $ty {}
		impl $( < $( $generics )* > )* Term<SingleSelection> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Array for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>      for $ty {}
		impl $( < $( $generics )* > )* Term<Datum>    for $ty {}
		impl $( < $( $generics )* > )* Term<Sequence> for $ty {}
		impl $( < $( $generics )* > )* Term<Array>    for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Sequence for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>      for $ty {}
		impl $( < $( $generics )* > )* Term<Sequence> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Stream for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>      for $ty {}
		impl $( < $( $generics )* > )* Term<Sequence> for $ty {}
		impl $( < $( $generics )* > )* Term<Stream>   for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? StreamSelection for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>             for $ty {}
		impl $( < $( $generics )* > )* Term<Sequence>        for $ty {}
		impl $( < $( $generics )* > )* Term<Stream>          for $ty {}
		impl $( < $( $generics )* > )* Term<StreamSelection> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Table for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>             for $ty {}
		impl $( < $( $generics )* > )* Term<Sequence>        for $ty {}
		impl $( < $( $generics )* > )* Term<Stream>          for $ty {}
		impl $( < $( $generics )* > )* Term<StreamSelection> for $ty {}
		impl $( < $( $generics )* > )* Term<Table>           for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Database for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>      for $ty {}
		impl $( < $( $generics )* > )* Term<Database> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Function<$args:ident, $output:ident> for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>      for $ty {}
		impl $( < $( $generics )* > )* Term<Function<$args, $output>> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Ordering for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>      for $ty {}
		impl $( < $( $generics )* > )* Term<Ordering> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? PathSpec for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Top>      for $ty {}
		impl $( < $( $generics )* > )* Term<PathSpec> for $ty {}
	};
	(impl $( ( $( $generics:tt )* ) )? Error for $ty:ty) => {
		impl $( < $( $generics )* > )* Term<Error> for $ty {}
	};
}

impl_types!(impl Null   for ());
impl_types!(impl Bool   for bool);
impl_types!(impl Number for i8);
impl_types!(impl Number for i16);
impl_types!(impl Number for i32);
impl_types!(impl Number for i64);
impl_types!(impl Number for i128);
impl_types!(impl Number for isize);
impl_types!(impl Number for u8);
impl_types!(impl Number for u16);
impl_types!(impl Number for u32);
impl_types!(impl Number for u64);
impl_types!(impl Number for u128);
impl_types!(impl Number for usize);
impl_types!(impl Number for f32);
impl_types!(impl Number for f64);
impl_types!(impl String for str);
impl_types!(impl ( 'a ) String for &'a str);
impl_types!(impl String for std::string::String);
impl_types!(impl ( 'a ) String for &'a std::string::String);
impl_types!(impl ( T: Term<Top> ) Array for [T]);
impl_types!(impl ( T: Term<Top> ) Array for std::vec::Vec<T>);
impl_types!(impl ( T: Term<Top> ) Object for std::collections::HashMap<std::string::String, T>);
impl_types!(impl ( T0: Term<Top> ) Array for (T0,));
impl_types!(impl ( T0: Term<Top>, T1: Term<Top> ) Array for (T0, T1));
impl_types!(impl ( T0: Term<Top>, T1: Term<Top>, T2: Term<Top> ) Array for (T0, T1, T2));
impl_types!(impl ( T0: Term<Top>, T1: Term<Top>, T2: Term<Top>, T3: Term<Top> ) Array for (T0, T1, T2, T3));
impl_types!(impl ( T0: Term<Top>, T1: Term<Top>, T2: Term<Top>, T3: Term<Top>, T4: Term<Top> ) Array for (T0, T1, T2, T3, T4));
impl_types!(impl ( T0: Term<Top>, T1: Term<Top>, T2: Term<Top>, T3: Term<Top>, T4: Term<Top>, T5: Term<Top> ) Array for (T0, T1, T2, T3, T4, T5));
impl_types!(impl ( T0: Term<Top>, T1: Term<Top>, T2: Term<Top>, T3: Term<Top>, T4: Term<Top>, T5: Term<Top>, T6: Term<Top> ) Array for (T0, T1, T2, T3, T4, T5, T6));
impl_types!(impl ( T0: Term<Top>, T1: Term<Top>, T2: Term<Top>, T3: Term<Top>, T4: Term<Top>, T5: Term<Top>, T6: Term<Top>, T7: Term<Top> ) Array for (T0, T1, T2, T3, T4, T5, T6, T7));

impl<T: Term<Datum>> Term<PathSpec> for T {}
//impl<T: Term<String>> Term<PathSpec> for T {}
//impl<T: Term<Object>> Term<PathSpec> for T {}
//impl<T: Term<Array>>  Term<PathSpec> for T {}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Ignored;

impl Serialize for Ignored {
	fn serialize<S: Serializer>(&self, _serializer: S) -> std::result::Result<S::Ok, S::Error> {
		unreachable!()
	}
}

impl Term<Top>                  for Ignored {}
impl Term<Datum>                for Ignored {}
impl Term<Null>                 for Ignored {}
impl Term<Bool>                 for Ignored {}
impl Term<Number>               for Ignored {}
impl Term<String>               for Ignored {}
impl Term<Object>               for Ignored {}
impl Term<SingleSelection>      for Ignored {}
impl Term<Array>                for Ignored {}
impl Term<Sequence>             for Ignored {}
impl Term<Stream>               for Ignored {}
impl Term<StreamSelection>      for Ignored {}
impl Term<Table>                for Ignored {}
impl Term<Database>             for Ignored {}
impl<A, O> Term<Function<A, O>> for Ignored {}
impl Term<Ordering>             for Ignored {}
impl Term<Error>                for Ignored {}

// TERM

#[derive(Clone, Debug)]
pub struct TermRepr<SIG, A: Serialize, O: Serialize, const N: usize> {
	_sig: PhantomData<SIG>,
	args: A,
	opts: Option<O>
}

impl<SIG, A: Serialize, O: Serialize, const N: usize> Serialize for TermRepr<SIG, A, O, N> {
	fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
		#[derive(Serialize)]
		struct Repr<'a, A, O>(usize, &'a A, #[serde(default, skip_serializing_if = "Option::is_none")] &'a Option<O>);
		Repr(N, &self.args, &self.opts).serialize(serializer)
	}
}

// FUNCTION

pub fn funcall<T: Term<Function<A, O>>, A, O, Args: Term<VarArg<Datum>>>(f: T, args: Args) -> TermRepr<fn(Function<A, O>, VarArg<Datum>) -> Datum, (T, Args), Ignored, 64> {
	TermRepr { _sig: PhantomData, args: (f, args), opts: None }
}

//impl_types!(impl ( T: Function<A, O>, A, O, Args: VarArg<dyn Datum> ) Datum for Term<(T, Args), (), 64>);

pub fn c<F: Closure<Args, T, Output = T>, Args, T: Serialize>(f: F) -> TermRepr<fn(Top, Top) -> Top, (TermRepr<fn(VarArg<Datum>) -> Array, (F::Input,), Ignored, 2>, F::Output), Ignored, 69> {
	f.evaluate()
}

pub fn closure<F: Closure<Args, T, Output = T>, Args, T: Serialize>(f: F) -> TermRepr<fn(Top, Top) -> Top, (TermRepr<fn(VarArg<Datum>) -> Array, (F::Input,), Ignored, 2>, F::Output), Ignored, 69> {
	f.evaluate()
}

pub trait Closure<Args, T: Serialize> {
	type Input:  Serialize;
	type Output: Serialize;
	
	fn evaluate(self) -> TermRepr<fn(Top, Top) -> Top, (TermRepr<fn(VarArg<Datum>) -> Array, (Self::Input,), Ignored, 2>, Self::Output), Ignored, 69>;
}

macro_rules! closure {
    ( $head:ident $(, $tail:ident )* ) => {
		closure!(@impl $head, $( $tail, )* );
		closure!( $( $tail ),* );
	};
	() => {};
	(@impl $( $ident:ident, )* ) => {
		impl<F: FnOnce( $( Var<$ident>, )* ) -> T, T: Term<Top>, $( $ident, )* > Closure< ( $( Var<$ident>, )* ), T> for F where $( Var<$ident>: Term<Datum>, )* {
			type Input  = ( $( Var<$ident>, )* );
			type Output = T;
			
			fn evaluate(self) -> TermRepr<fn(Top, Top) -> Top, (TermRepr<fn(VarArg<Datum>) -> Array, (Self::Input,), Ignored, 2>, Self::Output), Ignored, 69> {
				let ( $( $ident, )* ) = ( $( Var::<$ident>::default(), )* );
				func(make_array( ( $( $ident, )* ) ), (self)( $( $ident, )* ))
			}
		}
	};
	(@count $ident:ident) => { 1 };
	(@foreach $ident:ident do $( $tt:tt )* ) => { $( $tt )* };
}

closure!(T0, T1, T2, T3, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);

// VAR

pub struct Var<T: ?Sized>(PhantomData<T>, usize);

impl<T: ?Sized> Default for Var<T> {
	fn default() -> Self {
		Self(PhantomData, QUERY_VAR_COUNTER
			.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
	}
}

impl<T: ?Sized> Serialize for Var<T> {
	fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
		var(self.1).serialize(serializer)
	}
}

impl<T: ?Sized> Clone for Var<T> {
	fn clone(&self) -> Self {
		Self(PhantomData, self.1)
	}
}

impl<T: ?Sized> Copy for Var<T> {}

impl_types!(impl Top                   for Var<Top>);
impl_types!(impl Datum                 for Var<Datum>);
impl_types!(impl Null                  for Var<Null>);
impl_types!(impl Bool                  for Var<Bool>);
impl_types!(impl Number                for Var<Number>);
impl_types!(impl String                for Var<String>);
impl_types!(impl Object                for Var<Object>);
impl_types!(impl SingleSelection       for Var<SingleSelection>);
impl_types!(impl Array                 for Var<Array>);
impl_types!(impl Sequence              for Var<Sequence>);
impl_types!(impl Stream                for Var<Stream>);
impl_types!(impl StreamSelection       for Var<StreamSelection>);
impl_types!(impl Table                 for Var<Table>);
impl_types!(impl Database              for Var<Database>);
impl_types!(impl (A, O) Function<A, O> for Var<Function<A, O>>);
impl_types!(impl Ordering              for Var<Ordering>);
impl_types!(impl PathSpec              for Var<PathSpec>);
impl_types!(impl Error                 for Var<Error>);

// VAR ARG

pub struct VarArg<T>(PhantomData<T>);

macro_rules! typed_var_arg {
	( $ty:ident ) => {
		typed_var_arg!($ty; T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, );
	};
	( Function; $head:ident, $( $tail:ident, )* ) => {
		impl< A, O, $head: Term<Function<A, O>>, $( $tail: Term<Function<A, O>>, )* > Term<VarArg<Function<A, O>>> for ( $head, $( $tail, )* ) {}
		typed_var_arg!( Function; $( $tail, )* );
	};
	( $ty:ident; $head:ident, $( $tail:ident, )* ) => {
		impl< $head: Term<$ty>, $( $tail: Term<$ty>, )* > Term<VarArg<$ty>> for ( $head, $( $tail, )* ) {}
		typed_var_arg!( $ty; $( $tail, )* );
	};
	( $ty:ident; ) => {}
}

typed_var_arg!(Top);
typed_var_arg!(Datum);
typed_var_arg!(Null);
typed_var_arg!(Bool);
typed_var_arg!(Number);
typed_var_arg!(String);
typed_var_arg!(Object);
typed_var_arg!(SingleSelection);
typed_var_arg!(Array);
typed_var_arg!(Sequence);
typed_var_arg!(Stream);
typed_var_arg!(StreamSelection);
typed_var_arg!(Table);
typed_var_arg!(Database);
typed_var_arg!(Function);
typed_var_arg!(Ordering);
typed_var_arg!(PathSpec);
typed_var_arg!(Error);

// EXPR

pub struct ExprObjBuilder(serde_json::Map<std::string::String, serde_json::Value>);

impl ExprObjBuilder {
	pub fn new() -> Self {
		Self::default()
	}
	
	pub fn field(&mut self, key: &str, val: impl Serialize) -> &mut Self {
		self.0.insert(key.to_string(), serde_json::to_value(val).unwrap());
		self
	}
	
	pub fn finish(&mut self) -> TermRepr<fn() -> Object, (), serde_json::Map<std::string::String, serde_json::Value>, 3> {
		TermRepr { _sig: PhantomData, args: (), opts: Some(std::mem::replace(&mut self.0, serde_json::Map::new())) }
	}
}

impl Default for ExprObjBuilder {
	fn default() -> Self {
		Self(serde_json::Map::new())
	}
}

#[derive(Clone, Debug, Default)]
pub struct ExprSeqBuilder(Vec<serde_json::Value>);

impl ExprSeqBuilder {
	pub fn new() -> Self {
		Self::default()
	}
	
	pub fn element(&mut self, val: impl Serialize) -> &mut Self {
		self.0.push(serde_json::to_value(val).unwrap());
		self
	}
	
	pub fn finish(&mut self) -> TermRepr<fn(VarArg<Datum>) -> Array, Vec<serde_json::Value>, Ignored, 2> {
		TermRepr { _sig: PhantomData, args: std::mem::take(&mut self.0), opts: None }
	}
}

/// Helper macro to create `expr`s from closures, control structures and serializable values
#[macro_export]
macro_rules! expr {
	( $( $key:ident: $val:expr ),* ) => {{
		$crate::reql::ExprObjBuilder::new()
			$( .field( stringify!($key), $val ) )*
			.finish()
	}};
	( $( $val:expr ),* ) => [{
		$crate::reql::ExprSeqBuilder::new()
			$( .element( $val ) )*
			.finish()
	}];
}

// CAST

pub struct Cast<T: Serialize, C: ?Sized>(T, PhantomData<C>);

impl<T: Serialize, C: ?Sized> Serialize for Cast<T, C> {
	fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
		self.0.serialize(serializer)
	}
}

pub trait CastInfix: Serialize + Sized {
	fn cast<T>(self) -> Cast<Self, T> {
		Cast(self, PhantomData)
	}
}

impl<T: Serialize> CastInfix for T {}

impl_types!(impl ( T: Serialize ) Top             for Cast<T, Top>);
impl_types!(impl ( T: Serialize ) Datum           for Cast<T, Datum>);
impl_types!(impl ( T: Serialize ) Null            for Cast<T, Null>);
impl_types!(impl ( T: Serialize ) Bool            for Cast<T, Bool>);
impl_types!(impl ( T: Serialize ) Number          for Cast<T, Number>);
impl_types!(impl ( T: Serialize ) String          for Cast<T, String>);
impl_types!(impl ( T: Serialize ) Object          for Cast<T, Object>);
impl_types!(impl ( T: Serialize ) SingleSelection for Cast<T, SingleSelection>);
impl_types!(impl ( T: Serialize ) Array           for Cast<T, Array>);
impl_types!(impl ( T: Serialize ) Sequence        for Cast<T, Sequence>);
impl_types!(impl ( T: Serialize ) Stream          for Cast<T, Stream>);
impl_types!(impl ( T: Serialize ) StreamSelection for Cast<T, StreamSelection>);
impl_types!(impl ( T: Serialize ) Table           for Cast<T, Table>);
impl_types!(impl ( T: Serialize ) Database        for Cast<T, Database>);
impl_types!(impl ( T: Serialize ) Ordering        for Cast<T, Ordering>);
impl_types!(impl ( T: Serialize ) PathSpec        for Cast<T, PathSpec>);
impl_types!(impl ( T: Serialize ) Error           for Cast<T, Error>);

// TERM

macro_rules! term {
	($id:expr => fn $name:ident ( $( $arg_name:ident: $arg_type:path ),* ) -> $output:ident ) => {
		pub fn $name<
			$( $arg_name: Term<$arg_type>, )*
		>(
			$(  $arg_name: $arg_name, )*
		) -> TermRepr<fn( $( $arg_type, )* ) -> $output, ( $(  $arg_name, )* ), Ignored, $id> {
			TermRepr { _sig: PhantomData, args: ( $( $arg_name, )* ), opts: None }
		}
		
		term!(@infix $id => fn $name( $( $arg_name: $arg_type ),* ) -> $output );
		impl_types!(impl ( $( $arg_name: Term<$arg_type>, )* ) $output for TermRepr<fn( $( $arg_type, )* ) -> $output, ( $( $arg_name, )* ), Ignored, $id>);
	};
	($id:expr => fn $name:ident(
		$( $arg_name:ident: $arg_type:path, )*
		@struct $opts:ident { $( $opt_name:ident: $opt_type:path ),* }
	) -> $output:ident ) => {
		#[derive(Clone, Debug, Default, Serialize)]
		pub struct $opts< $( $opt_name: Term<$opt_type> = Ignored, )* > {
			$( pub $opt_name: Option<$opt_name>, )*
		}
		
		pub fn $name<
			$( $arg_name: Term<$arg_type>, )*
			$( $opt_name: Term<$opt_type>, )*
		>(
			$(  $arg_name: $arg_name, )*
			opts: Option<$opts< $( $opt_name, )* >>
		) -> TermRepr<fn( $( $arg_type, )* ) -> $output, ( $(  $arg_name, )* ), $opts< $( $opt_name, )* >, $id> {
			TermRepr { _sig: PhantomData, args: ( $( $arg_name, )* ), opts }
		}
		
		term!(@infix $id => fn $name( $( $arg_name: $arg_type, )* @struct $opts { $( $opt_name: $opt_type ),* }) -> $output );
		impl_types!(impl ( $( $arg_name: Term<$arg_type>, )* $( $opt_name: Term<$opt_type>, )* ) $output for TermRepr<fn( $( $arg_type, )* ) -> $output, ( $( $arg_name, )* ), $opts< $(  $opt_name, )* >, $id>);
	};
	(@infix $id:expr => fn $name:ident(
		$head_arg_name:ident: $head_arg_type:path
		$(, $arg_name:ident: $arg_type:path )*
	) -> $output:ident ) => {
		pub trait $name: Term<$head_arg_type> + Sized {
			fn $name<
				$( $arg_name: Term<$arg_type>, )*
			>(
				self,
				$( $arg_name: $arg_name, )*
			) -> TermRepr<fn( $head_arg_type, $( $arg_type, )* ) -> $output, ( Self, $( $arg_name, )* ), Ignored, $id> {
				TermRepr { _sig: PhantomData, args: (self, $( $arg_name, )* ), opts: None }
			}
		}
		
		impl<T: Term<$head_arg_type>> $name for T {}
	};
	(@infix $id:expr => fn $name:ident(
		$head_arg_name:ident: $head_arg_type:path,
		$( $arg_name:ident: $arg_type:path, )*
		@struct $opts:ident { $( $opt_name:ident: $opt_type:path ),* }
	) -> $output:ident ) => {
		pub trait $name: Term<$head_arg_type> + Sized {
			fn $name<
				$( $arg_name: Term<$arg_type>, )*
				$( $opt_name: Term<$opt_type>, )*
			>(
				self,
				$(  $arg_name: $arg_name, )*
				opts: Option<$opts< $( $opt_name, )* >>
			) -> TermRepr<fn( $head_arg_type, $( $arg_type, )* ) -> $output, ( Self, $(  $arg_name, )* ), $opts< $( $opt_name, )* >, $id> {
				TermRepr { _sig: PhantomData, args: (self, $( $arg_name, )* ), opts }
			}
		}
		
		impl<T: Term<$head_arg_type>> $name for T {}
	};
	(@infix $( $tt:tt )*) => {};
}

term!(2  => fn make_array(v: VarArg<Datum>) -> Array);

pub fn make_obj<T: Serialize>(v: T) -> TermRepr<fn() -> Object, (), T, 3> {
	TermRepr { _sig: PhantomData, args: (), opts: Some(v) }
}

impl_types!(impl ( T: Serialize ) Object for TermRepr<fn() -> Object, (), T, 3>);

term!(10 => fn var(r: Number) -> Datum);
term!(11 => fn javascript(code: String, @struct JavaScriptOptions { timeout: Number } ) -> Datum);
term!(12 => fn error() -> Error);
term!(12 => fn error_from_string(s: String) -> Error);
term!(13 => fn implicit_var() -> Datum);
term!(14 => fn db(s: String) -> Database);
term!(15 => fn table(db: Database, name: String, @struct TableOptions { read_mode: String, id_format: String }) -> Table);
term!(16 => fn get(t: Table, s: Datum) -> SingleSelection);
term!(17 => fn eq(a: Datum, b: Datum) -> Bool);
term!(18 => fn ne(a: Datum, b: Datum) -> Bool);
term!(19 => fn lt(a: Datum, b: Datum) -> Bool);
term!(20 => fn le(a: Datum, b: Datum) -> Bool);
term!(21 => fn gt(a: Datum, b: Datum) -> Bool);
term!(22 => fn ge(a: Datum, b: Datum) -> Bool);
term!(23 => fn not(b: Bool) -> Bool);
term!(24 => fn add(a: Number, b: Number) -> Number);
term!(25 => fn sub(a: Number, b: Number) -> Number);
term!(26 => fn mul(a: Number, b: Number) -> Number);
term!(27 => fn div(a: Number, b: Number) -> Number);
term!(28 => fn r#mod(a: Datum, b: Datum) -> Number);
term!(29 => fn append(a: Array, v: Datum) -> Array);
term!(30 => fn slice(s: Sequence, from: Number, to: Number) -> Sequence);
term!(31 => fn get_field(o: Object, s: String) -> Datum);
term!(32 => fn has_fields(o: Object, p: VarArg<PathSpec>) -> Bool);
term!(33 => fn pluck(seq_or_obj: Top, p: VarArg<PathSpec>) -> Top);
term!(34 => fn without(seq_or_obj: Top, p: VarArg<PathSpec>) -> Top);
term!(35 => fn merge(o: VarArg<Object>) -> Object);
term!(35 => fn merge_fn(o: Object, f: Function<Datum, Datum>) -> Object);
term!(35 => fn merge_seq(s: Sequence) -> Sequence);
term!(35 => fn merge_seq_fn(s: Sequence, f: Function<Datum, Datum>) -> Sequence);
term!(36 => fn between_deprecated(s: StreamSelection, a: Datum, b: Datum) -> Bool);
term!(37 => fn reduce(s: Sequence, f: Function<(Datum, Datum), Datum>) -> Datum);
term!(38 => fn map(s: Sequence, f: Function<Datum, Datum>) -> Sequence);
term!(39 => fn filter(s: Sequence, o: Object) -> Sequence);
term!(39 => fn filter_fn(s: Sequence, f: Function<Datum, Bool>) -> Sequence);
term!(40 => fn concat_map(s: Sequence, f: Function<Datum, Sequence>) -> Sequence);
term!(41 => fn order_by(s: Sequence, o: Ordering, @struct OrderByOptions { index: String } ) -> Sequence);
term!(42 => fn distinct(s: Sequence) -> Sequence);
term!(43 => fn count(s: Sequence) -> Number);
term!(43 => fn count_datum(s: Sequence, v: Datum) -> Number);
term!(43 => fn count_fn(s: Sequence, f: Function<Datum, Bool>) -> Number);
term!(44 => fn union(s: VarArg<Sequence>) -> Sequence);
term!(45 => fn nth(s: Sequence, n: Number) -> Datum);
term!(48 => fn inner_join(a: Sequence, b: Sequence, f: Function<(Datum, Datum), Bool>) -> Sequence);
term!(49 => fn outer_join(a: Sequence, b: Sequence, f: Function<(Datum, Datum), Bool>) -> Sequence);
term!(50 => fn eq_join(a: Sequence, s: String, b: Sequence, @struct EqJoinOptions { index: String }) -> Sequence);
term!(51 => fn coerce_to(v: Top, t: String) -> Top);
term!(52 => fn type_of(v: Top) -> String);
term!(53 => fn update_with(s: StreamSelection, f: Function<Object, Object>, @struct UpdateWithOptions { non_atomic: Bool, durability: String, return_changes: Bool } ) -> Object);
term!(53 => fn update_one_with(s: SingleSelection, f: Function<Object, Object>, @struct UpdateOneWithOptions { non_atomic: Bool, durability: String, return_changes: Bool } ) -> Object);
term!(53 => fn update(s: StreamSelection, o: Object, @struct UpdateOptions { non_atomic: Bool, durability: String, return_changes: Bool } ) -> Object);
term!(53 => fn update_one(s: SingleSelection, o: Object, @struct UpdateOneOptions { non_atomic: Bool, durability: String, return_changes: Bool } ) -> Object);
term!(54 => fn delete(s: StreamSelection, @struct DeleteOptions { durability: String, return_cahnges: Bool } ) -> Object);
term!(54 => fn delete_one(s: SingleSelection) -> Object);
term!(55 => fn reaplce(s: StreamSelection, f: Function<Datum, Datum>, @struct ReplaceOptions { non_atomic: Bool, durability: String, return_changes: Bool } ) -> Object);
term!(55 => fn replace_one(s: SingleSelection, f: Function<Datum, Datum>, @struct ReplaceOneOptions { non_atomic: Bool, durability: String, return_changes: Bool } ) -> Object);
term!(56 => fn insert(t: Table, o: Object, @struct InsertOptions { conflict: String, durability: String, return_changes: Bool } ) -> Object);
term!(56 => fn insert_seq(t: Table, s: Sequence, @struct InsertSeqOptions { conflict: String, durability: String, return_changes: Bool } ) -> Object);
term!(57 => fn db_create(name: String) -> Object);
term!(58 => fn db_drop(name: String) -> Object);
term!(59 => fn db_list() -> Array);
term!(60 => fn table_create(name: String, @struct TableCreateOptions { primary_key: String, shards: Number, replicas: Datum, primary_replica_tag: String }) -> Object);
term!(60 => fn table_create_db(db: Database, name: String, @struct TableCreateDbOptions { primary_key: String, shards: Number, replicas: Datum, primary_replica_tag: String }) -> Object);
term!(61 => fn table_drop(name: String) -> Object);
term!(61 => fn table_drop_db(db: Database, name: String) -> Object);
term!(62 => fn table_list() -> Array);
term!(62 => fn table_list_db(db: Database) -> Array);

term!(65 => fn branch(c: Bool, a: Top, b: Top) -> Top);
term!(66 => fn or(v: VarArg<Bool>) -> Bool);
term!(67 => fn and(v: VarArg<Bool>) -> Bool);
term!(68 => fn for_each(s: Sequence, f: Function<Datum, Null>) -> Object);
term!(69 => fn func(args: Top, t: Top) -> Top);
term!(70 => fn skip(s: Sequence, v: Number) -> Sequence);
term!(71 => fn limit(s: Sequence, v: Number) -> Sequence);
term!(72 => fn zip(s: Sequence) -> Sequence);
term!(73 => fn asc(s: String) -> Ordering);
term!(74 => fn desc(s: String) -> Ordering);
term!(75 => fn index_create(t: Table, s: String, @struct IndexCreateOptions { multi: Bool, geo: Bool }) -> Object);
term!(75 => fn index_create_fn(t: Table, s: String, f: Function<Datum, Datum>, @struct IndexCreateFnOptions { multi: Bool, geo: Bool }) -> Object);
term!(76 => fn index_drop(t: Table, s: String) -> Object);
term!(77 => fn index_list(t: Table) -> Array);
term!(78 => fn get_all(t: Table, d: VarArg<Datum>) -> Array);
term!(79 => fn info(v: Top) -> Object);
term!(80 => fn prepend(a: Array, v: Datum) -> Array);
term!(81 => fn sample(s: Sequence, n: Number) -> Sequence);
term!(82 => fn insert_at(a: Array, i: Number, v: Datum) -> Array);
term!(83 => fn delete_at(a: Array, i: Number) -> Array);
term!(83 => fn delete_range(a: Array, o: Number, e: Number) -> Array);
term!(84 => fn change_at(a: Array, i: Number, v: Datum) -> Array);
term!(85 => fn splice_at(a: Array, i: Number, v: Array) -> Array);
term!(86 => fn is_empty(s: Sequence) -> Bool);
term!(87 => fn offset_of(s: Sequence, v: Datum) -> Sequence);
term!(87 => fn offset_of_fn(s: Sequence, f: Function<Datum, Bool>) -> Sequence);
term!(88 => fn set_insert(a: Array, v: Datum) -> Array);
term!(89 => fn set_intersection(a: Array, b: Array) -> Array);
term!(90 => fn set_union(a: Array, b: Array) -> Array);
term!(91 => fn set_difference(a: Array, b: Array) -> Array);
term!(92 => fn default_(a: Top, b: Top) -> Top);
term!(93 => fn contains_one(s: Sequence, v: Datum) -> Bool);
term!(93 => fn contains_one_fn(s: Sequence, v: Function<Datum, Bool>) -> Bool);
term!(93 => fn contains(s: Sequence, v: VarArg<Datum>) -> Bool);
term!(93 => fn contains_fn(s: Sequence, v: VarArg<Function<Datum, Bool>>) -> Bool);
term!(94 => fn keys(o: Object) -> Array);
term!(95 => fn difference(a: Array, b: Array) -> Array);
term!(96 => fn with_fields(s: Sequence, p: VarArg<PathSpec>) -> Sequence);
term!(97 => fn matches(s: String, pattern: String) -> Datum);
term!(98 => fn json(s: String) -> Datum);
term!(99 => fn iso8601(s: String) -> Datum);
term!(100 => fn to_iso8601(v: Datum) -> String);
term!(101 => fn epoch_time(n: Number) -> Datum);
term!(102 => fn to_epoch_time(v: Datum) -> Number);
term!(103 => fn now() -> Datum);
term!(104 => fn in_timezone(v: Datum, tz: String) -> Datum);
term!(105 => fn during(a: Datum, b: Datum, c: Datum) -> Bool);
term!(106 => fn date(d: Datum) -> Datum);
term!(107 => fn monday() -> Number);
term!(108 => fn tuesday() -> Number);
term!(109 => fn wednesday() -> Number);
term!(110 => fn thursday() -> Number);
term!(111 => fn friday() -> Number);
term!(112 => fn saturday() -> Number);
term!(113 => fn sunday() -> Number);
term!(114 => fn january() -> Number);
term!(115 => fn february() -> Number);
term!(116 => fn march() -> Number);
term!(117 => fn april() -> Number);
term!(118 => fn may() -> Number);
term!(119 => fn june() -> Number);
term!(120 => fn july() -> Number);
term!(121 => fn august() -> Number);
term!(122 => fn september() -> Number);
term!(123 => fn october() -> Number);
term!(124 => fn november() -> Number);
term!(125 => fn december() -> Number);
term!(126 => fn time_of_day(t: Datum) -> Number);
term!(127 => fn timezone(t: Datum) -> String);
term!(128 => fn year() -> Number);
term!(129 => fn month() -> Number);
term!(130 => fn day() -> Number);
term!(131 => fn dayofweek() -> Number);
term!(132 => fn dayofyear() -> Number);
term!(133 => fn hour() -> Number);
term!(134 => fn minutes() -> Number);
term!(135 => fn seconds() -> Number);
term!(136 => fn to_date(year: Number, month: Number, day: Number, timezone: String) -> Datum);
term!(136 => fn to_date_time(year: Number, month: Number, day: Number, hour: Number, minute: Number, second: Number, timezone: String) -> Datum);
term!(137 => fn literal() -> Top);
term!(137 => fn literal_json(s: String) -> Top);
term!(138 => fn sync(t: Table) -> Object);
term!(139 => fn index_status(t: Table, indexes: VarArg<String>) -> Array);
term!(140 => fn index_wait(t: Table, indexes: VarArg<String>) -> Array);
term!(141 => fn to_uppercase(s: String) -> String);
term!(142 => fn to_lowercase(s: String) -> String);
term!(143 => fn create_object(s: String, d: VarArg<Datum>) -> Object);
term!(144 => fn group(s: Sequence, v: String) -> Sequence);
term!(144 => fn group_fn(s: Sequence, f: Function<Datum, Datum>) -> Sequence);
term!(145 => fn sum(s: Sequence, v: String) -> Sequence);
term!(145 => fn sum_fn(s: Sequence, f: Function<Datum, Datum>) -> Sequence);
term!(146 => fn avg(s: Sequence, v: String) -> Sequence);
term!(146 => fn avg_fn(s: Sequence, f: Function<Datum, Datum>) -> Sequence);
term!(147 => fn min(s: Sequence, v: String) -> Sequence);
term!(147 => fn min_fn(s: Sequence, f: Function<Datum, Datum>) -> Sequence);
term!(148 => fn max(s: Sequence, v: String) -> Sequence);
term!(148 => fn max_fn(s: Sequence, f: Function<Datum, Datum>) -> Sequence);
term!(149 => fn split_whitespace(s: String) -> Array);
term!(149 => fn split(s: String, p: String) -> Array);
term!(149 => fn splitn(s: String, p: String, n: Number) -> Array);
term!(149 => fn splitn_whitespace(s: String, null: Null, n: Number) -> Array);
term!(150 => fn ungroup(v: Top) -> Array);
term!(151 => fn random(start: Number, end: Number, @struct RandomOptions { float: Bool } ) -> Datum);
term!(152 => fn changes(t: Table) -> Stream);
term!(153 => fn http(url: String) -> String);
term!(154 => fn args(a: Array) -> Top);

term!(169 => fn uuid() -> Datum);

term!(174 => fn config(db_or_table: Top) -> Object);
term!(175 => fn status(db_or_table: Top) -> Object);
term!(176 => fn reconfigure(db_or_table: Top, @struct ReconfigureOptions { shards: Number, replicas: Object, primary_replica_tag: String, nonvoting_replica_tags: String, emergency_repair: String, dry_run: Bool }) -> Object);
term!(177 => fn wait(db_or_table: Top) -> Object);
term!(179 => fn rebalance(db_or_table: Top) -> Object);
term!(180 => fn minval() -> Number);
term!(181 => fn maxval() -> Number);
term!(182 => fn between(s: StreamSelection, a: Datum, b: Datum, @struct BetweenOptions { index: String, right_bound: String, left_bound: String } ) -> StreamSelection);
term!(183 => fn floor(v: Number) -> Number);
term!(184 => fn ceil(v: Number) -> Number);
term!(185 => fn round(v: Number) -> Number);
term!(186 => fn values(o: Object) -> Array);
term!(187 => fn fold(s: Sequence, base: Datum, f: Function<(Datum, Datum), Datum>, @struct FoldOptions { emit: Function<(Datum, Datum, Datum), Array>, final_emit: Function<Datum, Array> } ) -> Sequence);
term!(188 => fn grant(db_or_table: Top) -> Object);
term!(189 => fn set_write_hook(t: Table, f: Function<(Object, Object, Object), Object>) -> Null);
//term!(190 => fn get_write_hook(t: Table) -> Function<(Object, Object, Object), Object>);
term!(191 => fn bit_and(a: Number, b: Number) -> Number);
term!(192 => fn bit_or(a: Number, b: Number) -> Number);
term!(193 => fn bit_xor(a: Number, b: Number) -> Number);
term!(194 => fn bit_not(v: Number) -> Number);
term!(195 => fn bit_sal(a: Number, b: Number) -> Number);
term!(196 => fn bit_sar(a: Number, b: Number) -> Number);

// GET FIELD AS

pub struct GetFieldAsTerm<T0: Term<Object>, T1: Term<String>, T: ?Sized>(TermRepr<fn(Object, String) -> Datum, (T0, T1), (), 31>, PhantomData<T>);

impl<T0: Term<Object>, T1: Term<String>, T: ?Sized> Serialize for GetFieldAsTerm<T0, T1, T> {
	fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>  {
		self.0.serialize(serializer)
	}
}

pub trait get_field_as: Term<Object> + Sized {
	fn get_field_as<T: ?Sized, T1: Term<String>>(self, s: T1) -> GetFieldAsTerm<Self, T1, T> {
		GetFieldAsTerm(TermRepr { _sig: PhantomData, args: (self, s), opts: None }, PhantomData)
	}
}

impl<T: Term<Object>> get_field_as for T {}

impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Top                  for GetFieldAsTerm<T0, T1, Top>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Datum                for GetFieldAsTerm<T0, T1, Datum>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Null                 for GetFieldAsTerm<T0, T1, Null>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Bool                 for GetFieldAsTerm<T0, T1, Bool>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Number               for GetFieldAsTerm<T0, T1, Number>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) String               for GetFieldAsTerm<T0, T1, String>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Object               for GetFieldAsTerm<T0, T1, Object>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) SingleSelection      for GetFieldAsTerm<T0, T1, SingleSelection>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Array                for GetFieldAsTerm<T0, T1, Array>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Sequence             for GetFieldAsTerm<T0, T1, Sequence>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Stream               for GetFieldAsTerm<T0, T1, Stream>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) StreamSelection      for GetFieldAsTerm<T0, T1, StreamSelection>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Table                for GetFieldAsTerm<T0, T1, Table>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Database             for GetFieldAsTerm<T0, T1, Database>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String>, A, O ) Function<A, O> for GetFieldAsTerm<T0, T1, Function<A, O>>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Ordering             for GetFieldAsTerm<T0, T1, Ordering>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) PathSpec             for GetFieldAsTerm<T0, T1, PathSpec>);
impl_types!(impl ( T0: Term<Object>, T1: Term<String> ) Error                for GetFieldAsTerm<T0, T1, Error>);

#[cfg(test)]
mod tests {
	use super::*;
	
	#[test]
	fn simple_term() {
		let v = serde_json::to_string(&db("blog")
			.table::<_, Ignored, Ignored>("users", None)
			.filter(crate::expr! { name: "Michel" }))
			.unwrap();
		
		assert_eq!(&v, "[39,[[15,[[14,[\"blog\"]],\"users\"]],{\"name\":\"Michel\"}]]");
	}
	
	#[test]
	fn function_term() {
		let v = serde_json::to_string(&c(|a: Var<Number>, b: Var<Number>| { a.add(b) })).unwrap();
		assert_eq!(&v, "[69,[[2,[1,2]],[24,[[10,[1]],[10,[2]]]]]]");
	}
}
