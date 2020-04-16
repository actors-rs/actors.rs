import React from 'react'
import {
  Wrapper,
  Container,
  MainImage,
  Headline,
  ButtonWrapper,
  ListBadges,
} from './styled'
import Image from './MainVisual.png'
import { ButtonLink } from '../../Atoms/Button'
import Badge from '../../Atoms/Badge'

const badges = [
  {
    badgeUrl: 'https://travis-ci.org/actors-rs/actors.rs',
    badgeSrc: 'https://travis-ci.org/actors-rs/actors.rs.svg?branch=master',
  },
  {
    badgeUrl: 'https://github.com/actors-rs/actors.rs/blob/master/LICENSE',
    badgeSrc: 'https://img.shields.io/badge/license-MIT-blue.svg',
  },
  {
    badgeUrl: 'https://crates.io/crates/actors-rs',
    badgeSrc: 'https://meritbadge.herokuapp.com/actors-rs',
  },
  {
    badgeUrl: 'https://docs.rs/riker',
    badgeSrc: 'https://docs.rs/actors-rs/badge.svg',
  },
  {
    badgeUrl: 'https://github.com/prettier/prettier',
    badgeSrc:
      'https://img.shields.io/badge/code_style-prettier-ff69b4.svg?style=flat-square',
  },
  {
    badgeUrl: 'https://github.com/pre-commit/pre-commit',
    badgeSrc:
      'https://img.shields.io/badge/pre--commit-enabled-brightgreen?logo=pre-commit&logoColor=white',
  },
]

const MainVisual = () => {
  return (
    <Wrapper>
      <Container>
        <MainImage src={Image} />
        <Headline>
          A Rust framework for building modern, concurrent and resilient
          applications
        </Headline>
        <ListBadges>
          {badges.map((badge, index) => (
            <Badge key={index} url={badge.badgeUrl} src={badge.badgeSrc} />
          ))}
        </ListBadges>
        <ButtonWrapper>
          <ButtonLink href="/book/actors.html" primary={'1'}>
            Get Started
          </ButtonLink>
          <ButtonLink href="/faq/">FAQ</ButtonLink>
        </ButtonWrapper>
      </Container>
    </Wrapper>
  )
}

export default MainVisual
