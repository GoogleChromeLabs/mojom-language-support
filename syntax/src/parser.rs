#[derive(Parser)]
#[grammar = "mojom.pest"]
pub(crate) struct MojomParser;

pub(crate) type Pairs<'a> = pest::iterators::Pairs<'a, Rule>;

pub(crate) fn consume_token(rule: Rule, pairs: &mut Pairs) {
    if pairs.next().unwrap().as_rule() != rule {
        unreachable!()
    }
}
