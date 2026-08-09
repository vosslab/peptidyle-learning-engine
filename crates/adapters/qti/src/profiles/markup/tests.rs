use super::*;

use crate::model::QtiImportLimits;
use crate::xml::parse_xml;

fn xml(source: &str) -> XmlNode {
    parse_xml("fixture.xml", source.as_bytes(), QtiImportLimits::default()).expect("XML parses")
}

#[test]
fn canvas_projects_mixed_inline_blocks_lists_code_and_entities() {
    let node = xml(
        "<mattext>One &amp;lt;two&amp;gt; &amp;amp; &lt;strong&gt;bold&lt;/strong&gt;&lt;p&gt;next \
        &lt;em&gt;bit&lt;/em&gt;&lt;br/&gt;last&lt;/p&gt;&lt;ul&gt;&lt;li&gt;first&lt;/li&gt;&lt;li&gt;\
        &lt;code&gt;x``y&lt;/code&gt;&lt;/li&gt;&lt;/ul&gt;</mattext>",
    );
    assert_eq!(
        project_canvas_mattext(&node, MarkupLimits::PROMPT).unwrap(),
        "One &lt;two&gt; &amp; **bold**\n\nnext *bit*  \nlast\n\n- first\n- ```x``y```"
    );
    let nested = xml("<mattext>&amp;lt;</mattext>");
    assert_eq!(
        project_canvas_mattext(&nested, MarkupLimits::PROMPT).unwrap(),
        "&lt;"
    );
}

#[test]
fn blackboard_preserves_direct_xml_order_without_html_reparse() {
    let node = xml(
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'><p>One \
        <strong>two</strong> three</p><ol><li>A</li><li>B</li></ol></itemBody>",
    );
    assert_eq!(
        project_blackboard_xhtml(&node, MarkupLimits::PROMPT).unwrap(),
        "One **two** three\n\n1. A\n2. B"
    );
    let literal = xml(
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'>\
        &lt;strong&gt;literal&lt;/strong&gt;</itemBody>",
    );
    assert_eq!(
        project_blackboard_xhtml(&literal, MarkupLimits::PROMPT).unwrap(),
        "&lt;strong&gt;literal&lt;/strong&gt;"
    );
    let pretty_list = xml(
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'><ul>\n  \
        <li>first</li>\n  <li>second</li>\n</ul></itemBody>",
    );
    assert_eq!(
        project_blackboard_xhtml(&pretty_list, MarkupLimits::PROMPT).unwrap(),
        "- first\n- second"
    );
    let spaced_blocks = xml(
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'>\n  <p>one</p>\n  \
        <p>two</p>\n</itemBody>",
    );
    assert_eq!(
        project_blackboard_xhtml(&spaced_blocks, MarkupLimits::PROMPT).unwrap(),
        "one\n\ntwo"
    );
}

#[test]
fn root_whitespace_does_not_change_block_markdown_but_inline_spaces_remain_semantic() {
    let canvas = xml("<mattext>\n  &lt;p&gt;one&lt;/p&gt;\n  &lt;p&gt;two&lt;/p&gt;\n</mattext>");
    assert_eq!(
        project_canvas_mattext(&canvas, MarkupLimits::PROMPT).unwrap(),
        "one\n\ntwo"
    );
    let blackboard = xml(
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'>one \n        <strong>two</strong></itemBody>",
    );
    assert_eq!(
        project_blackboard_xhtml(&blackboard, MarkupLimits::PROMPT).unwrap(),
        "one **two**"
    );
    let canvas_plain = xml("<mattext>    plain    </mattext>");
    assert_eq!(
        project_canvas_mattext(&canvas_plain, MarkupLimits::PROMPT).unwrap(),
        "plain"
    );
    let blackboard_plain =
        xml("<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'>    plain    </itemBody>");
    assert_eq!(
        project_blackboard_xhtml(&blackboard_plain, MarkupLimits::PROMPT).unwrap(),
        "plain"
    );
}

#[test]
fn refuses_unsupported_markup_attributes_and_xml_events() {
    for source in [
        "<mattext>&lt;table/&gt;</mattext>",
        "<mattext>&lt;style&gt;x&lt;/style&gt;</mattext>",
        "<mattext>&lt;img src='x'/&gt;</mattext>",
        "<mattext>&lt;image/&gt;</mattext>",
        "<mattext>&lt;audio/&gt;</mattext>",
        "<mattext>&lt;video/&gt;</mattext>",
        "<mattext>&lt;svg/&gt;</mattext>",
        "<mattext>&lt;math/&gt;</mattext>",
        "<mattext>&lt;u&gt;x&lt;/u&gt;</mattext>",
        "<mattext>&lt;sub&gt;x&lt;/sub&gt;</mattext>",
        "<mattext>&lt;sup&gt;x&lt;/sup&gt;</mattext>",
        "<mattext>&lt;a href='https://example.test'&gt;x&lt;/a&gt;</mattext>",
        "<mattext>&lt;p class='x'&gt;x&lt;/p&gt;</mattext>",
        "<mattext>&lt;p class='x' class='y'&gt;x&lt;/p&gt;</mattext>",
        "<mattext><!-- note --></mattext>",
        "<mattext><?pi x?></mattext>",
        "<mattext>&lt;p&gt;x&lt;/div&gt;</mattext>",
        "<mattext>&lt;p&gt;x</mattext>",
        "<mattext>&lt;p&gt;one&lt;p&gt;two</mattext>",
        "<mattext>&lt;strong/&gt;</mattext>",
        "<mattext>&lt;/br&gt;</mattext>",
        "<mattext>a &lt;</mattext>",
        "<mattext>&lt;!DOCTYPE html&gt;</mattext>",
    ] {
        let node = xml(source);
        assert!(
            project_canvas_mattext(&node, MarkupLimits::PROMPT).is_err(),
            "{source}"
        );
    }
    assert_eq!(
        tokenize_canvas_html("a\0b", MarkupLimits::PROMPT),
        Err(QtiMarkupError::UnsupportedMarkup)
    );
    for source in [
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'><img/></itemBody>",
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'><p onclick='x'>x</p></itemBody>",
        "<itemBody xmlns='urn:foreign'><p>x</p></itemBody>",
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'><p xmlns='urn:foreign'>x</p></itemBody>",
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'><p id='x'>x</p></itemBody>",
        "<itemBody xmlns='http://www.imsglobal.org/xsd/imsqti_v2p1'><!--x--></itemBody>",
    ] {
        let node = xml(source);
        assert!(
            project_blackboard_xhtml(&node, MarkupLimits::PROMPT).is_err(),
            "{source}"
        );
    }
}

#[test]
fn enforces_input_token_depth_and_output_limits() {
    let exact_input = xml("<mattext>abc</mattext>");
    let input_exact = MarkupLimits {
        max_input_bytes: 3,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&exact_input, input_exact).unwrap(),
        "abc"
    );
    let input_plus_one = MarkupLimits {
        max_input_bytes: 2,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&exact_input, input_plus_one),
        Err(QtiMarkupError::InputLimit)
    );

    let token_exact = MarkupLimits {
        max_tokens: 2,
        ..MarkupLimits::PROMPT
    };
    assert!(tokenize_canvas_html("x", token_exact).is_ok());
    let token_plus_one = MarkupLimits {
        max_tokens: 1,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        tokenize_canvas_html("x", token_plus_one),
        Err(QtiMarkupError::TokenLimit)
    );

    let exact_output = MarkupLimits {
        max_output_chars: 5,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&xml("<mattext>abcde</mattext>"), exact_output).unwrap(),
        "abcde"
    );
    let output_plus_one = MarkupLimits {
        max_output_chars: 4,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&xml("<mattext>abcde</mattext>"), output_plus_one),
        Err(QtiMarkupError::OutputLimit)
    );

    let node = xml("<mattext>abcdef</mattext>");
    let input = MarkupLimits {
        max_input_bytes: 5,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&node, input),
        Err(QtiMarkupError::InputLimit)
    );
    let token = MarkupLimits {
        max_tokens: 1,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&node, token),
        Err(QtiMarkupError::TokenLimit)
    );
    let output = MarkupLimits {
        max_output_chars: 5,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&node, output),
        Err(QtiMarkupError::OutputLimit)
    );
    let deep =
        xml("<mattext>&lt;strong&gt;&lt;strong&gt;x&lt;/strong&gt;&lt;/strong&gt;</mattext>");
    let nesting_exact = MarkupLimits {
        max_nesting: 3,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&deep, nesting_exact).unwrap(),
        "****x****"
    );
    let nesting = MarkupLimits {
        max_nesting: 2,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&deep, nesting),
        Err(QtiMarkupError::NestingLimit)
    );
}

#[test]
fn sink_never_retains_more_than_its_token_limit() {
    let sink = HtmlTokenSink::new(1);
    let _ = sink.process_token(Token::EOFToken, 1);
    assert_eq!(sink.tokens.borrow().len(), 1);
    assert!(!sink.overflow.get());
    let _ = sink.process_token(Token::EOFToken, 1);
    assert_eq!(sink.tokens.borrow().len(), 1);
    assert!(sink.overflow.get());
}

#[test]
fn escaped_text_refuses_before_output_growth_exceeds_the_limit() {
    let count = MarkupLimits::PROMPT.max_output_chars;
    let source = format!("<mattext>{}</mattext>", "&amp;".repeat(count));
    let node = xml(&source);
    let limits = MarkupLimits {
        max_tokens: 100_000,
        ..MarkupLimits::PROMPT
    };
    assert_eq!(
        project_canvas_mattext(&node, limits),
        Err(QtiMarkupError::OutputLimit)
    );
}

#[test]
fn exposes_only_the_fixed_safe_markup_diagnostic() {
    let _diagnostic =
        QtiMarkupError::InvalidNamespace.safe_diagnostic(QtiSafeDiagnosticLocation::Prompt);
}
