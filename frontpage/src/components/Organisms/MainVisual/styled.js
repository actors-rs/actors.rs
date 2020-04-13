import styled from 'styled-components'

export const Wrapper = styled.div`
  background-image: linear-gradient(150deg, #2f0070 0%, #10d2fb 100%);
  position: relative;
  z-index: 0;
`

export const Container = styled.div`
  max-width: 1000px;
  width: 96%;
  margin: auto;
  padding: 120px 0 60px;
`

export const Headline = styled.p`
  font-family: Lato;
  font-weight: 700;
  font-size: 30px;
  color: #ffffff;
  letter-spacing: 0.7px;
  line-height: 36px;
  margin-bottom: 20px;
  max-width: 540px;
  width: 90%;
  position: relative;
`

export const MainImage = styled.img`
  position: absolute;
  right: 0;
  bottom: 0;
  max-width: 90%;
  max-height: 96%;
  width: auto;
  height: auto;
`

export const ButtonWrapper = styled.div`
  display: flex;
  width: 210px;
  justify-content: space-between;
  margin-bottom: 20px;
  position: relative;
`

export const ListBadges = styled.div`
  display: flex;
  margin-bottom: 1rem;
  position: relative;
  z-index: 1;
  & > * {
    margin-left: 0.5rem;
    &:first-child {
      margin-left: 0;
    }
  }
`
