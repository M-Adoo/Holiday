use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use std::collections::{BTreeMap, HashSet};
use syn::{
  bracketed,
  parse::{Parse, ParseStream},
  parse_quote, parse_quote_spanned,
  punctuated::{Pair, Punctuated},
  spanned::Spanned,
  token::{self, Brace, Comma},
  visit_mut::VisitMut,
  Expr, Ident, Path,
};
mod widget_gen;
use crate::error::{DeclareError, DeclareWarning};
mod builtin_fields;
pub use builtin_fields::*;
pub use widget_gen::WidgetGen;

use super::{
  kw,
  widget_macro::{is_expr_keyword, EXPR_FIELD, EXPR_WIDGET},
  DeclareCtx, Id, ObjectUsed, Result, ScopeUsedInfo, UsedPart,
};

#[derive(Debug)]
pub struct DeclareWidget {
  pub path: Path,
  brace_token: Brace,
  // the name of this widget specified by `id` attr.
  pub named: Option<Id>,
  fields: Punctuated<DeclareField, Comma>,
  pub builtin: BuiltinFieldWidgets,
  pub children: Vec<DeclareWidget>,
}

#[derive(Clone, Debug)]
pub struct DeclareField {
  pub skip_nc: Option<SkipNcAttr>,
  pub member: Ident,
  pub colon_token: Option<token::Colon>,
  pub expr: Expr,
  pub used_name_info: ScopeUsedInfo,
}

#[derive(Clone, Debug)]
pub struct SkipNcAttr {
  pound_token: token::Pound,
  bracket_token: token::Bracket,
  skip_nc_meta: kw::skip_nc,
}

macro_rules! assign_uninit_field {
  ($self: ident.$name: ident, $field: ident) => {
    assign_uninit_field!($self.$name, $field, $name)
  };
  ($left: expr, $right: ident, $name: ident) => {
    if $left.is_none() {
      $left = Some($right);
      Ok(())
    } else {
      Err(syn::Error::new(
        $right.span(),
        format!("`{}` declare more than once", stringify!($name)).as_str(),
      ))
    }
  };
}

pub(crate) use assign_uninit_field;

impl ToTokens for SkipNcAttr {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.pound_token.to_tokens(tokens);
    self.bracket_token.surround(tokens, |tokens| {
      self.skip_nc_meta.to_tokens(tokens);
    })
  }
}

impl ToTokens for DeclareField {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.member.to_tokens(tokens);
    if self.colon_token.is_some() {
      self.colon_token.to_tokens(tokens);
      self.expr.to_tokens(tokens);
    }
  }
}

impl Spanned for DeclareWidget {
  fn span(&self) -> Span { self.path.span().join(self.brace_token.span).unwrap() }
}

impl Parse for DeclareWidget {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let content;
    let mut widget = DeclareWidget {
      path: input.parse()?,
      brace_token: syn::braced!(content in input),
      named: None,
      fields: Punctuated::default(),
      builtin: BuiltinFieldWidgets::default(),
      children: vec![],
    };
    loop {
      if content.is_empty() {
        break;
      }

      if content.peek(Ident) && content.peek2(token::Brace) {
        widget.children.push(content.parse()?);
      } else {
        let f: DeclareField = content.parse()?;
        if !widget.children.is_empty() {
          return Err(syn::Error::new(
            f.span(),
            "Field should always declare before children.",
          ));
        }
        widget.fields.push(f);
        if !content.is_empty() {
          content.parse::<token::Comma>()?;
        }
      }
    }

    check_duplicate_field(&widget.fields)?;
    pick_fields_by(&mut widget.fields, |p| {
      let p = if p.value().is_id_field() {
        widget.named = Some(Id::from_field_pair(p)?);
        None
      } else if let Some(ty) = BuiltinFieldWidgets::is_builtin_field(&widget.path, p.value()) {
        widget.builtin.fill_as_builtin_field(ty, p.into_value());
        None
      } else {
        Some(p)
      };
      Ok(p)
    })?;

    Ok(widget)
  }
}

impl Parse for SkipNcAttr {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let pound_token = input.parse()?;
    let content;
    let bracket_token = bracketed!(content in input);
    Ok(Self {
      pound_token,
      bracket_token,
      skip_nc_meta: content.parse()?,
    })
  }
}

impl Parse for DeclareField {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let skip_nc = try_parse_skip_nc(input)?;
    let member: Ident = input.parse()?;
    let colon_token: Option<_> = input.parse()?;
    let expr = if colon_token.is_some() {
      input.parse()?
    } else {
      Expr::Path(syn::ExprPath {
        attrs: Vec::new(),
        qself: None,
        path: Path::from(member.clone()),
      })
    };

    Ok(DeclareField {
      skip_nc,
      member,
      colon_token,
      expr,
      used_name_info: ScopeUsedInfo::default(),
    })
  }
}

impl DeclareField {
  pub fn used_part(&self) -> Option<UsedPart> {
    self
      .used_name_info
      .used_part(Some(&self.member), self.skip_nc.is_some())
  }

  pub fn is_id_field(&self) -> bool {
    let mem = &self.member;
    syn::parse2::<kw::id>(quote! {#mem}).is_ok()
  }
}

pub fn try_parse_skip_nc(input: ParseStream) -> syn::Result<Option<SkipNcAttr>> {
  if input.peek(token::Pound) {
    Ok(Some(input.parse()?))
  } else {
    Ok(None)
  }
}

impl DeclareCtx {
  pub fn visit_declare_widget_mut(&mut self, w: &mut DeclareWidget) {
    let mut ctx = self.stack_push();
    let DeclareWidget { path, fields, builtin, children, .. } = w;

    if is_expr_keyword(path) {
      if fields.len() != 1 || fields[0].member != EXPR_FIELD {
        let spans = fields.iter().map(|f| f.member.span().unwrap()).collect();
        let error = DeclareError::ExprWidgetInvalidField(spans).into_compile_error();
        fields.clear();
        fields.push(parse_quote! { expr: #error});
      } else {
        let expr_field = fields.first_mut().unwrap();
        let origin_expr = expr_field.expr.clone();
        ctx.visit_declare_field_mut(expr_field);

        let upstream = expr_field
          .used_name_info
          .all_widgets()
          .map(|objs| upstream_tokens(objs, quote! {raw_change_stream}));
        if let Some(upstream) = upstream {
          expr_field.expr = parse_quote_spanned! { origin_expr.span() =>
            move |cb: &mut dyn FnMut(Widget)| IntoExprGen::<_>::into_expr_gen(#origin_expr, cb)
          };

          // we convert the field expr to a closure, revisit again.
          expr_field.used_name_info.take();
          ctx.visit_declare_field_mut(expr_field);

          *path = parse_quote_spanned! { path.span() => #path::<_> };
          if !fields.trailing_punct() {
            fields.push_punct(Comma::default());
          }
          fields.push(parse_quote! {upstream: #upstream});
        } else {
          *path = parse_quote_spanned! { path.span() => ConstExprWidget<_> };
          assert!(expr_field.used_name_info.is_empty())
        }
      }
    } else {
      fields
        .iter_mut()
        .for_each(|f| ctx.visit_declare_field_mut(f));
    }

    ctx.visit_builtin_field_widgets(builtin);

    children
      .iter_mut()
      .for_each(|c| ctx.visit_declare_widget_mut(c))
  }

  pub fn visit_declare_field_mut(&mut self, f: &mut DeclareField) {
    self.visit_ident_mut(&mut f.member);
    self.visit_expr_mut(&mut f.expr);

    f.used_name_info = self.take_current_used_info();
  }

  pub fn visit_builtin_field_widgets(&mut self, builtin: &mut BuiltinFieldWidgets) {
    builtin.visit_builtin_fields_mut(self);
  }
}

impl DeclareWidget {
  pub fn host_and_builtin_widgets_tokens<'a>(
    &'a self,
    name: &'a Ident,
    ctx: &'a DeclareCtx,
  ) -> impl Iterator<Item = (Ident, TokenStream)> + '_ {
    let Self { path: ty, fields, .. } = self;
    let gen = WidgetGen::new(ty, name, fields.iter(), false);
    let host = gen.gen_widget_tokens(ctx);
    let builtin = self.builtin.widget_tokens_iter(name, ctx);
    std::iter::once((name.clone(), host)).chain(builtin)
  }

  pub fn before_generate_check(&self) -> Result<()> {
    self
      .traverses_widget()
      .try_for_each(|w| w.builtin.key_follow_check())
  }

  pub fn warnings(&self) -> impl Iterator<Item = DeclareWarning> + '_ {
    self
      .fields
      .iter()
      .chain(self.builtin.all_builtin_fields())
      .filter(|f| self.named.is_none() || f.used_name_info.all_widgets().is_none())
      .filter_map(|f| {
        f.skip_nc
          .as_ref()
          .map(|attr| DeclareWarning::NeedlessSkipNc(attr.span().unwrap()))
      })
      .chain(self.children.iter().flat_map(|c| {
        let iter: Box<dyn Iterator<Item = DeclareWarning>> = Box::new(c.warnings());
        iter
      }))
  }

  /// return follow relationship of the named widgets,it is a key-value map,
  /// schema like
  /// ``` ascii
  /// {
  ///   widget_name: [field, {depended_widget: [position]}]
  /// }
  /// ```
  pub fn analyze_object_dependencies(&self) -> BTreeMap<Ident, ObjectUsed> {
    let mut follows: BTreeMap<Ident, ObjectUsed> = BTreeMap::new();
    self.traverses_widget().for_each(|w| {
      if let Some(name) = w.name() {
        w.builtin.collect_builtin_widget_follows(name, &mut follows);

        let w_follows: ObjectUsed = w.fields.iter().flat_map(|f| f.used_part()).collect();

        if !w_follows.is_empty() {
          follows.insert(name.clone(), w_follows);
        }
      }
    });

    follows
  }

  pub(crate) fn is_expr_widget(&self) -> bool {
    self
      .path
      .segments
      .first()
      .map_or(false, |s| s.ident == EXPR_WIDGET)
  }

  pub fn traverses_widget(&self) -> impl Iterator<Item = &DeclareWidget> {
    let children: Box<dyn Iterator<Item = &DeclareWidget>> =
      Box::new(self.children.iter().flat_map(|w| w.traverses_widget()));

    std::iter::once(self).chain(children)
  }

  pub fn name(&self) -> Option<&Ident> { self.named.as_ref().map(|id| &id.name) }
}

pub fn upstream_tokens<'a>(
  used_widgets: impl Iterator<Item = &'a Ident> + Clone,
  stream_name: TokenStream,
) -> TokenStream {
  let upstream = used_widgets.clone().map(|w| {
    quote_spanned! { w.span() =>  #w.#stream_name() }
  });
  if used_widgets.count() > 1 {
    quote! {  observable::from_iter([#(#upstream),*]).merge_all(usize::MAX) }
  } else {
    quote! { #(#upstream)* }
  }
}

pub fn check_duplicate_field(fields: &Punctuated<DeclareField, Comma>) -> syn::Result<()> {
  let mut sets = HashSet::<&Ident, ahash::RandomState>::default();
  for f in fields {
    if !sets.insert(&f.member) {
      return Err(syn::Error::new(
        f.member.span(),
        format!("`{}` declare more than once", f.member.to_string()).as_str(),
      ));
    }
  }
  Ok(())
}

pub fn pick_fields_by(
  fields: &mut Punctuated<DeclareField, Comma>,
  mut f: impl FnMut(Pair<DeclareField, Comma>) -> syn::Result<Option<Pair<DeclareField, Comma>>>,
) -> syn::Result<()> {
  let coll = std::mem::take(fields);
  for p in coll.into_pairs() {
    if let Some(p) = f(p)? {
      let (field, comma) = p.into_tuple();
      fields.push(field);
      if let Some(comma) = comma {
        fields.push_punct(comma);
      }
    }
  }
  Ok(())
}
